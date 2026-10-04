import assert from 'node:assert/strict';
import { publicationIdentity } from './catalog-publication-timing.mjs';

// A cooperative opponent, but genuine catalog shuffles, draws and Verified
// commands throughout. No checkpoint injection or private-deck reordering.
export const runVerifiedNinjutsu = options => runVerifiedReturnCost({ ...options, mechanic: 'ninjutsu' });
export const runVerifiedSneak = options => runVerifiedReturnCost({ ...options, mechanic: 'sneak' });

async function runVerifiedReturnCost({ pages, row, seat, waitForFullUiPair, samePublishedStep, mechanic, maxSearchTurn = 40 }) {
  assert.ok(Number.isSafeInteger(maxSearchTurn) && maxSearchTurn >= 1 && maxSearchTurn <= 100);
  const [host, guest] = pages;
  const label = mechanic === 'sneak' ? 'Sneak' : 'Ninjutsu';
  const subject = mechanic === 'sneak' ? 'Oroku Saki, Shredder Rising' : 'Kaito, Bane of Nightmares';
  const carrier = 'Dream Beavers';
  let activated = false;
  let returned = false;
  let carrierId = null;
  let castCarrier = false;
  let returnedHandCount = null;
  let completedAt = null;
  const unstartedAttemptTurns = new Set();
  row.mechanicMaxSearchTurn = maxSearchTurn;
  for (let i = 0; i < Math.max(1800, (maxSearchTurn + 2) * 45); i++) {
    const readyStarted = performance.now();
    const pair = await waitForFullUiPair(host, guest, (a, b) => {
      const current = a.state.decision?.player === 0 ? a : b;
      return a.multiplayer.lastAppliedSequence === b.multiplayer.lastAppliedSequence
        && !a.multiplayer.pendingVerification && !b.multiplayer.pendingVerification
        && samePublishedStep(a, b) && current.state.decision
        && (current.state.decision.kind !== 'priority' || current.state.priority_analysis_complete);
    }, `Verified ${label} acting menu ready`, 30000);
    const actionMenuReadyObservationMs = performance.now() - readyStarted;
    const actor = pair.host.state.decision.player;
    const current = actor === 0 ? pair.host : pair.guest;
    const state = current.state;
    assert.ok(state.turn_number <= (completedAt === null ? maxSearchTurn : completedAt + 2),
      `${label} natural draw search or following turn check exceeded its bound`);
    const own = state.players[seat];
    if (activated && !returned && completedAt === null && carrierId != null
      && [pair.host, pair.guest].every(view => view.state.decisionDetail.kind === 'priority'
        && view.state.stack_preview.length === 0
        && view.state.players[seat].battlefield.some(card => card.id === carrierId))
      && (seat === 0 ? pair.host : pair.guest).state.players[seat].hand_cards.some(card => card.name === subject)) {
      // A transcript sequence can publish before React renders its new
      // decision. Confirm this is genuinely unchanged in both engine workers.
      const authoritative = await Promise.all(pages.map(page => page.evaluate(() =>
        window.__ironsmithE2E.runtimeState())));
      if (!authoritative.every(view => view?.decision?.kind === 'priority' && view.stack_size === 0
        && view.players[seat].battlefield.some(card => card.id === carrierId))
        || !authoritative[seat].players[seat].hand_cards.some(card => card.name === subject)) continue;
      // Activated abilities may be offered before the mana window proves
      // affordability. An unchanged priority state is not a paid return cost.
      (row.unstartedReturnCostAttempts ||= []).push({ turn: state.turn_number,
        seq: current.multiplayer.lastAppliedSequence, carrierId, subject });
      unstartedAttemptTurns.add(state.turn_number);
      activated = false;
      returnedHandCount = null;
    }
    if (activated && !returned && carrierId != null) {
      const ownerView = seat === 0 ? pair.host : pair.guest;
      // The engine may automatically choose the only legal return-cost object.
      // Assert its actual zone change rather than requiring a redundant prompt.
      returned = [pair.host, pair.guest].every(view =>
        !view.state.players[seat].battlefield.some(c => c.id === carrierId))
        && ownerView.state.players[seat].hand_cards.filter(c => c.name === carrier).length === returnedHandCount;
    }
    if (completedAt === null && activated && returned && own.battlefield.some(c => c.name === subject)) {
      for (const view of [pair.host, pair.guest]) {
        const entered = view.state.players[seat].battlefield.find(c => c.name === subject);
        assert.ok(entered?.tapped, `${label} creature enters tapped on both peers`);
        assert.ok(view.state.combat?.attackers.some(a => a.creature === entered.id
          && a.target.kind === 'player' && a.target.player === 1 - seat), 'enters attacking opponent');
        assert.ok(!view.state.combat.attackers.some(a => a.creature === carrierId), 'returned attacker left combat');
        assert.equal(view.state.stack_preview.length, 0, `${label} finished resolving`);
      }
      const actingView = seat === 0 ? pair.host : pair.guest;
      assert.equal(actingView.state.players[seat].hand_cards.filter(c => c.name === carrier).length, returnedHandCount, 'return cost adds exactly one attacker to hand');
      row.reachedTurn = state.turn_number;
      completedAt = state.turn_number;
      row[mechanic] = { seat, castCarrier, activated, returned, carrierId, correctness: 'passed',
        resolutionTurn: completedAt, resolutionCombat: pair.host.state.combat };
    }
    if (completedAt !== null && state.turn_number >= completedAt + 2) {
      for (const view of [pair.host, pair.guest]) {
        assert.ok(view.state.players[seat].battlefield.some(c => c.name === subject),
          `${subject} survives the following natural turn transitions`);
      }
      row.reachedTurn = state.turn_number;
      return;
    }
    const decision = state.decisionDetail;
    const actions = state.decisionActions || [];
    if (actor === seat && decision.kind === 'priority'
      && row.naturalDrawTrace?.at(-1)?.turn !== state.turn_number) {
      (row.naturalDrawTrace ||= []).push({ turn: state.turn_number,
        hand: own.hand_cards, battlefield: own.battlefield, actions });
    }
    const names = new Map(state.players[actor].hand_cards.map(c => [c.id, c.name]));
    let command;
    if (decision.kind === 'priority') {
      const returnCost = actor === seat && !activated && !unstartedAttemptTurns.has(state.turn_number)
        && actions.find(a => mechanic === 'sneak'
        ? a.action_ref.kind === 'cast_spell' && names.get(a.action_ref.spell_id) === subject
          && a.action_ref.casting_method?.kind === 'alternative'
        : a.action_ref.kind === 'activate_ability' && names.get(a.action_ref.source) === subject);
      const creature = actor === seat && !castCarrier && actions.find(a =>
        a.action_ref.kind === 'cast_spell' && names.get(a.action_ref.spell_id) === carrier);
      const land = !activated && actions.find(a => a.action_ref.kind === 'play_land');
      const action = returnCost || land || creature || actions.find(a =>
        ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(a.action_ref.kind));
      assert.ok(action, 'offered progress action');
      command = { type: 'priority_action', action_ref: action.action_ref };
      if (returnCost) {
        activated = true;
        returnedHandCount = own.hand_cards.filter(c => c.name === carrier).length + 1;
      }
      if (creature && action === creature) castCarrier = true;
    } else if (decision.kind === 'mana_payment') {
      command = { type: 'mana_payment', response: { action: 'confirm', plan_id: decision.plan_id,
        request_hash: decision.request_hash, required_source_ids: [], excluded_source_ids: [], preserved_source_ids: [] } };
    } else if (decision.kind === 'attackers') {
      const attacker = actor === seat && !activated && own.hand_cards.some(c => c.name === subject)
        && decision.attacker_options.find(a => a.creature_name === carrier);
      if (attacker) carrierId = attacker.creature;
      command = { type: 'declare_attackers', declarations: attacker
        ? [{ creature: attacker.creature, target: { kind: 'player', player: 1 - seat } }] : [] };
    } else if (decision.kind === 'blockers') {
      command = { type: 'declare_blockers', declarations: [] };
    } else if (decision.kind === 'select_objects') {
      const legal = state.decisionCandidates.filter(c => c.legal !== false);
      if (/^Scry 1\b/.test(decision.description || '') && decision.min === 0) {
        // Keep the one privately viewed card on top; the normal Verified
        // selection path still authenticates this choice.
        command = { type: 'select_objects', object_ids: [] };
      } else if (activated && !returned && actor === seat) {
        const attacker = legal.find(c => c.id === carrierId);
        assert.ok(attacker, `${label} offers the declared attacker as its return cost`);
        command = { type: 'select_objects', object_ids: [attacker.id] };
        returned = true;
      } else {
        assert.equal(decision.reason, 'Discard', `unexpected object selection: ${JSON.stringify(decision)}`);
        const count = Number(/^Discard (\d+) card/.exec(decision.description || '')?.[1]);
        assert.ok(Number.isSafeInteger(count) && count > 0);
        // Keep one mechanic card and one carrier. Duplicate copies may be discarded.
        const protectedIds = new Set([subject, carrier].map(name => legal.find(c => c.name === name)?.id));
        const ranked = [...legal].sort((a, b) => Number(protectedIds.has(a.id)) - Number(protectedIds.has(b.id)));
        assert.ok(ranked.length >= count);
        command = { type: 'select_objects', object_ids: ranked.slice(0, count).map(c => c.id) };
      }
    } else if (decision.kind === 'select_options') {
      command = { type: 'select_options', option_indices: [state.decisionOptions.find(o => o.legal !== false).index] };
    }
    assert.ok(command, `Unhandled ${label} decision ${JSON.stringify(decision)}`);
    if (command.type === 'mana_payment' || command.action_ref?.kind === 'activate_ability'
      || (command.action_ref?.kind === 'cast_spell' && command.action_ref.casting_method?.kind === 'alternative')
      || (completedAt !== null && decision.reason === 'Discard')) {
      (row.mechanicCheckpoints ||= []).push({ seq: current.multiplayer.lastAppliedSequence,
        command, checkpoints: await Promise.all(pages.map(page => page.evaluate(() => window.__ironsmithE2E.publicCheckpoint()))) });
    }
    const before = performance.now();
    const previousSequence = current.multiplayer.lastAppliedSequence;
    const submission = await pages[actor].evaluate(async command => {
      const started = performance.now();
      await window.__ironsmithE2E.submitMultiplayerCommand(command);
      return { submittedAt: performance.timeOrigin + started, submissionMs: performance.now() - started };
    }, command);
    const publicationStart = performance.now();
    const result = await waitForFullUiPair(host, guest, (a, b) =>
      a.multiplayer.lastAppliedSequence > previousSequence
      && a.multiplayer.lastAppliedSequence === b.multiplayer.lastAppliedSequence
      && !a.multiplayer.pendingVerification && !b.multiplayer.pendingVerification && samePublishedStep(a, b),
    `both peers verify ${label} progression`, 30000);
    row.interactions.push({ actor, seq: previousSequence + 1, command, ...submission,
      turn: state.turn_number, priorityRevision: state.priority_revision, actionMenuReadyObservationMs,
      observedSequence: result.host.multiplayer.lastAppliedSequence,
      publicationObservationMs: performance.now() - publicationStart,
      callerThroughVerificationMs: performance.now() - before,
      publicationTargets: [result.host, result.guest].map(view => ({
        sequence: view.multiplayer.lastAppliedSequence, identity: publicationIdentity(view.state),
      })) });
    console.log(`[Verified ${label} seat ${seat}] turn ${state.turn_number} seq ${previousSequence + 1}: ${command.action_ref?.kind || command.type}`);
  }
  assert.fail(`${label} natural game did not resolve within command limit`);
}
