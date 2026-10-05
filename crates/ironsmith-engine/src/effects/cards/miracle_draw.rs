//! The optional Miracle reveal belongs to the original draw, before any
//! replacement-added program or subsequent card draw can change its context.
use crate::decision::DecisionMaker;
use crate::decisions::context::{SelectOptionsContext, SelectableOption, ViewCardsContext};
use crate::effects::ExecutionError;
use crate::events::other::{DrawnMiracleInstance, DrawnMiraclePrice, MiracleDrawDecision, MiracleDrawOpportunity, MiracleInstanceIdentity, RevealedMiracle};
use crate::game_state::GameState;
use crate::grant::Grantable;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::zone::Zone;

pub(crate) struct CompletedMiracleDraw {
    pub cards: Vec<ObjectId>,
    pub miracle: Option<MiracleDrawDecision>,
}
fn grants_miracle(grant: &Grantable) -> bool {
    matches!(grant,
        Grantable::DerivedAlternativeCast(crate::grant::DerivedAlternativeCast::MiracleFromCardManaCostReducedBy { .. })
        | Grantable::AlternativeCast(crate::alternative_cast::AlternativeCastingMethod::Miracle { .. }))
}

fn capture_opportunity(game: &GameState, card: ObjectId, player: PlayerId) -> Result<MiracleDrawOpportunity, ExecutionError> {
    let object = game.object(card).filter(|object| object.zone == Zone::Hand && object.owner == player)
        .ok_or(ExecutionError::InvalidTarget)?;
    let mut instances = object.alternative_casts.iter().enumerate().filter_map(|(alternative_index, method)|
        method.miracle_cost().map(|cost| DrawnMiracleInstance {
            identity: MiracleInstanceIdentity::Intrinsic { alternative_index }, granting_source: card,
            price: DrawnMiraclePrice::Fixed(cost.clone()),
        })).collect::<Vec<_>>();
    for grant in game.effect_store.grant_registry.get_grants_for_card(game, card, Zone::Hand, player) {
        let price = match &grant.grantable {
            Grantable::DerivedAlternativeCast(crate::grant::DerivedAlternativeCast::MiracleFromCardManaCostReducedBy { reduction }) =>
                DrawnMiraclePrice::ReducedManaCost {
                    mana_cost: game.try_current_characteristics(card).map_err(ExecutionError::ContinuousDiscovery)?
                        .ok_or_else(|| ExecutionError::IncompleteEvidence("drawn Miracle card has no current characteristics".into()))?.mana_cost,
                    other_face_mana_cost: crate::decision::spell_view_for_split_other_half_cast(game, object)
                        .and_then(|face| face.mana_cost_owned()),
                    generic_reduction: *reduction,
                },
            Grantable::AlternativeCast(crate::alternative_cast::AlternativeCastingMethod::Miracle { cost }) =>
                DrawnMiraclePrice::Fixed(cost.clone()),
            _ => continue,
        };
        let identity = grant.permission_identity.clone().ok_or_else(||
            ExecutionError::IncompleteEvidence("drawn Miracle grant has no exact permission identity".into()))?;
        instances.push(DrawnMiracleInstance { identity: MiracleInstanceIdentity::Granted(identity),
            granting_source: grant.source.source_id(), price });
    }
    Ok(MiracleDrawOpportunity { card, stable_id: object.stable_id, player, instances })
}

fn choose_miracle_as_drawn(
    game: &mut GameState,
    card: ObjectId,
    player: PlayerId,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<Option<MiracleDrawDecision>, ExecutionError> {
    game.refresh_continuous_state().map_err(ExecutionError::ContinuousDiscovery)?;
    if game.hidden_identity_is_private(card) {
        // This eligibility question consults public grant/deck information
        // only. The owner and peers holding placeholders open the same window.
        let potential_grant = game.effect_store.grant_registry.active_grants(game).iter()
            .any(|grant| grant.player == player && grant.zone == Zone::Hand && grants_miracle(&grant.grantable));
        if !potential_grant && !game.hidden_draw_reveal_players().contains(&player) {
            return Ok(Some(MiracleDrawDecision::Declined));
        }
        let Some(opened) = game.reveal_private_hidden_cards_publicly(decision_maker, player, card,
            &[card], "You may reveal the first card you drew this turn to use Miracle", true)
        else { return Ok(None); };
        if !opened.contains(&card) { return Ok(Some(MiracleDrawDecision::Declined)); }
        game.refresh_continuous_state().map_err(ExecutionError::ContinuousDiscovery)?;
    }
    if game.is_hidden_card_placeholder(card) {
        return Err(ExecutionError::IncompleteEvidence("the drawn Miracle identity was not opened on this peer".into()));
    }
    let opportunity = capture_opportunity(game, card, player)?;
    if opportunity.instances.is_empty() { return Ok(Some(MiracleDrawDecision::Declined)); }
    let mut options = vec![SelectableOption::new(0, "Do not reveal using Miracle")];
    options.extend(opportunity.instances.iter().enumerate().map(|(index, instance)| {
        let price = match &instance.price {
            DrawnMiraclePrice::Fixed(cost) => cost.to_oracle(),
            DrawnMiraclePrice::ReducedManaCost { generic_reduction, .. } => format!("mana cost reduced by {{{generic_reduction}}}"),
        };
        SelectableOption::new(index + 1, format!("Reveal using Miracle ({price})"))
    }));
    // A different ability may already have opened the card. That public
    // knowledge never selects one of these linked Miracle instances.
    let question = SelectOptionsContext::new(player, Some(card), "Choose a Miracle reveal", options, 1, 1);
    let choice = decision_maker.decide_options(game, &question);
    if decision_maker.awaiting_choice() { return Ok(None); }
    let [index] = choice.as_slice() else { return Err(ExecutionError::Impossible("expected one Miracle reveal choice".into())); };
    if *index == 0 { return Ok(Some(MiracleDrawDecision::Declined)); }
    let instance = opportunity.instances.get(index - 1).cloned().ok_or_else(|| ExecutionError::Impossible("unknown Miracle reveal instance".into()))?;
    let object = game.object(card).ok_or(ExecutionError::InvalidTarget)?;
    let drawn_snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(object, game);
    for viewer in game.players.iter().map(|player| player.id).collect::<Vec<_>>() {
        let view = ViewCardsContext::new(viewer, player, Some(card), Zone::Hand, "Reveal drawn card for Miracle").with_public(true);
        decision_maker.view_cards(game, viewer, &[card], &view);
    }
    Ok(Some(MiracleDrawDecision::Revealed(RevealedMiracle {
        card: opportunity.card, stable_id: opportunity.stable_id, player: opportunity.player,
        instance, drawn_snapshot,
    })))
}

/// The caller owns the draw checkpoint. Pending input leaves no committed
/// original and therefore permits neither added programs nor the next draw.
pub(crate) fn draw_cards_with_miracle_window(
    game: &mut GameState,
    player: PlayerId,
    count: usize,
    first_this_turn: bool,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<CompletedMiracleDraw, ExecutionError> {
    let mut completed = CompletedMiracleDraw { cards: Vec::new(), miracle: None };
    for _ in 0..count {
        let drawn = game.draw_cards_with_dm(player, 1, decision_maker);
        if decision_maker.awaiting_choice() { return Ok(completed); }
        if let Some(&card) = drawn.first()
            && first_this_turn && completed.cards.is_empty()
        {
            let Some(decision) = choose_miracle_as_drawn(game, card, player, decision_maker)? else { return Ok(completed); };
            completed.miracle = Some(decision);
        }
        completed.cards.extend(drawn);
    }
    Ok(completed)
}


pub(crate) fn miracle_reveal_event(
    game: &mut GameState,
    drawn: &crate::events::CardsDrawnEvent,
    parent: crate::provenance::ProvNodeId,
) -> Option<crate::triggers::TriggerEvent> {
    let MiracleDrawDecision::Revealed(proof) = drawn.miracle.as_ref()? else { return None; };
    let provenance = game.provenance_graph_mut().alloc_child_event(parent, crate::events::EventKind::CardRevealed);
    Some(crate::triggers::TriggerEvent::new_with_provenance(crate::events::CardRevealedEvent::new(
        proof.player, proof.card, Zone::Hand, Some(proof.card), Some(proof.drawn_snapshot.clone())), provenance))
}
