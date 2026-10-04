import assert from 'node:assert/strict';
import { publicationIdentity } from './catalog-publication-timing.mjs';

// Exact catalog decks, genuine shuffles/draws and authenticated commands. The
// opponent cooperates; no checkpoint injection, card insertion or deck sorting.
export async function runVerifiedBroodscale({ pages, row, seat, waitForFullUiPair,
  samePublishedStep, maxSearchTurn = 100, iterations = 8 }) {
  assert.ok([0, 1].includes(seat));
  assert.ok(Number.isSafeInteger(iterations) && iterations > 0 && iterations <= 128);
  const broodName = 'Basking Broodscale', bladeName = 'Blade of the Bloodchief';
  const spawnName = 'Eldrazi Spawn';
  let stage = 'setup', brood, blade, sacrificed, completed = 0, finishedTurn, initialLife;
  const count = (cards, name) => cards.filter(card => card.name === name)
    .reduce((sum, card) => sum + (card.count || 1), 0);
  for (let step = 0; step < (maxSearchTurn + 3) * 55 + iterations * 20; step++) {
    const readyStarted = performance.now();
    const pair = await waitForFullUiPair(...pages, (a, b) => {
      const current = a.state.decision?.player === 0 ? a : b;
      return a.multiplayer.lastAppliedSequence === b.multiplayer.lastAppliedSequence
        && !a.multiplayer.pendingVerification && !b.multiplayer.pendingVerification
        && samePublishedStep(a, b) && current.state.decision
        && (current.state.decision.kind !== 'priority' || current.state.priority_analysis_complete);
    }, 'Verified Broodscale acting menu ready', 30000);
    const actionMenuReadyObservationMs = performance.now() - readyStarted;
    const actor = pair.host.state.decision.player;
    const current = actor === 0 ? pair.host : pair.guest;
    const state = current.state, decision = state.decisionDetail;
    const own = state.players[seat], actions = state.decisionActions || [];
    assert.ok(state.turn_number <= (finishedTurn === undefined ? maxSearchTurn : finishedTurn + 2),
      'Natural Broodscale search/following turn exceeded its bound');
    const main = actor === seat && state.active_player === seat
      && state.phase === 'first main phase' && !state.stack_preview.length;
    if (stage.endsWith('-pending') && decision.kind === 'priority' && !state.stack_preview.length) {
      const checkpoints = await Promise.all(pages.map(page => page.evaluate(() => window.__ironsmithE2E.publicCheckpoint())));
      const runtimes = await Promise.all(pages.map(page => page.evaluate(() => window.__ironsmithE2E.runtimeState())));
      for (const [index, cp] of checkpoints.entries()) {
        const board = cp.objects.filter(object => object.zone === 'battlefield' && object.controller === seat);
        const source = board.find(object => Number(object.id) === brood);
        assert.ok(source, 'Original Broodscale remains on both peers');
        if (stage === 'equip-pending') {
          const equipment = board.find(object => Number(object.id) === blade);
          assert.deepEqual(equipment?.attachedTo, { kind: 'object', object: brood },
            'Blade is attached to the selected Broodscale');
        } else {
          const expected = completed + (stage === 'loop-pending' ? 1 : 0);
          const tokens = board.filter(object => object.identity?.name === spawnName);
          assert.equal(tokens.length, 1, 'One Spawn is replaced per completed iteration');
          if (stage === 'loop-pending') assert.notEqual(Number(tokens[0].id), sacrificed, 'Sacrificed token identity is gone');
          const card = runtimes[index].players[seat].battlefield.find(object => Number(object.id) === brood);
          assert.equal(card?.counters.find(counter => counter.kind === '+1/+1')?.amount, expected + 1);
          assert.equal(runtimes[index].players[seat].mana_pool.colorless, expected);
          assert.deepEqual(runtimes[index].players.map(player => player.life), initialLife);
        }
      }
      if (stage === 'equip-pending') stage = 'adapt';
      else {
        if (stage === 'loop-pending') completed++;
        stage = 'loop';
        if (completed === iterations) {
          stage = 'following-turn'; finishedTurn = state.turn_number;
          row.broodscale = { seat, iterations, completed, brood, blade,
            resolutionTurn: finishedTurn, correctness: 'loop oracle passed', checkpoints };
        }
      }
    }
    if (stage === 'following-turn' && state.turn_number >= finishedTurn + 2) {
      for (const page of pages) {
        const runtime = await page.evaluate(() => window.__ironsmithE2E.runtimeState());
        assert.equal(runtime.players[seat].mana_pool.colorless, 0, 'Floating mana emptied at the natural phase boundary');
        assert.equal(count(runtime.players[seat].battlefield, spawnName), 1);
        const card = runtime.players[seat].battlefield.find(card => Number(card.id) === brood);
        assert.equal(card?.counters.find(counter => counter.kind === '+1/+1')?.amount, iterations + 1);
        assert.deepEqual(runtime.players.map(player => player.life), initialLife);
      }
      row.broodscale.correctness = 'passed'; row.reachedTurn = state.turn_number; return;
    }
    let command;
    if (decision.kind === 'priority') {
      let action;
      if (stage === 'setup' && main) {
        const forestCount = own.battlefield.filter(card => card.name === 'Forest' && !card.tapped)
          .reduce((sum, card) => sum + (card.count || 1), 0);
        if (count(own.battlefield, broodName) === 1 && count(own.battlefield, bladeName) === 1 && forestCount >= 4) {
          brood = Number(own.battlefield.find(card => card.name === broodName).id);
          blade = Number(own.battlefield.find(card => card.name === bladeName).id);
          initialLife = state.players.map(player => player.life); stage = 'equip';
        } else {
          const names = new Map(own.hand_cards.map(card => [Number(card.id), card.name]));
          action = actions.find(action => action.action_ref.kind === 'play_land'
            && names.get(Number(action.action_ref.land_id)) === 'Forest')
            || actions.find(action => action.action_ref.kind === 'cast_spell'
              && [broodName, bladeName].includes(names.get(Number(action.action_ref.spell_id)))
              && count(own.battlefield, names.get(Number(action.action_ref.spell_id))) === 0);
        }
      }
      if (main && ['equip', 'adapt', 'loop'].includes(stage)) {
        const source = stage === 'equip' ? blade : stage === 'adapt' ? brood
          : Number(own.battlefield.find(card => card.name === spawnName)?.id);
        action = actions.find(action => Number(action.action_ref.source) === source
          && action.action_ref.kind === (stage === 'loop' ? 'activate_mana_ability' : 'activate_ability'));
        assert.ok(action, `Offered ${stage} action required`);
        if (stage === 'loop') sacrificed = source;
        stage += '-pending';
      }
      action ||= actions.find(action => ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(action.action_ref.kind));
      assert.ok(action, 'Offered progression action required');
      command = { type: 'priority_action', action_ref: action.action_ref };
    } else if (decision.kind === 'mana_payment') {
      command = { type: 'mana_payment', response: { action: 'confirm', plan_id: decision.plan_id,
        request_hash: decision.request_hash, required_source_ids: [], excluded_source_ids: [], preserved_source_ids: [] } };
    } else if (decision.kind === 'targets') {
      assert.equal(stage, 'equip-pending');
      assert.ok(decision.requirements[0].legal_targets.some(target => target.kind === 'object' && Number(target.object) === brood));
      command = { type: 'select_targets', targets: [{ kind: 'object', object: brood }] };
    } else if (decision.kind === 'select_options') {
      const legal = state.decisionOptions.filter(option => option.legal !== false);
      const options = /Order triggered abilities/.test(decision.description || '') ? legal.map(option => option.index)
        : [legal.find(option => /^yes$/i.test(option.description))?.index];
      assert.ok(options.every(index => index !== undefined), `Unexpected options ${JSON.stringify(decision)}`);
      command = { type: 'select_options', option_indices: options };
    } else if (decision.kind === 'select_objects') {
      assert.equal(decision.reason, 'Discard');
      const legal = state.decisionCandidates.filter(card => card.legal !== false);
      const required = Number(/^Discard (\d+) card/.exec(decision.description || '')?.[1]);
      assert.ok(Number.isSafeInteger(required) && required > 0);
      const protectedIds = new Set();
      if (actor === seat && stage === 'setup') {
        for (const name of [broodName, bladeName]) if (!count(own.battlefield, name)) {
          const card = legal.find(card => card.name === name); if (card) protectedIds.add(card.id);
        }
        if (count(own.battlefield, 'Forest') < 4) for (const card of legal.filter(card => card.name === 'Forest').slice(0, 4 - count(own.battlefield, 'Forest'))) protectedIds.add(card.id);
      }
      const ranked = [...legal].sort((a, b) => Number(protectedIds.has(a.id)) - Number(protectedIds.has(b.id)));
      assert.ok(ranked.length >= required);
      command = { type: 'select_objects', object_ids: ranked.slice(0, required).map(card => card.id) };
    } else if (decision.kind === 'attackers') command = { type: 'declare_attackers', declarations: [] };
    else if (decision.kind === 'blockers') command = { type: 'declare_blockers', declarations: [] };
    assert.ok(command, `Unhandled Verified Broodscale decision ${JSON.stringify(decision)}`);
    const before = performance.now(), previousSequence = current.multiplayer.lastAppliedSequence;
    const submission = await pages[actor].evaluate(async command => {
      const started = performance.now(); await window.__ironsmithE2E.submitMultiplayerCommand(command);
      return { submittedAt: performance.timeOrigin + started, submissionMs: performance.now() - started };
    }, command);
    const publicationStart = performance.now();
    const result = await waitForFullUiPair(...pages, (a, b) => a.multiplayer.lastAppliedSequence > previousSequence
      && a.multiplayer.lastAppliedSequence === b.multiplayer.lastAppliedSequence
      && !a.multiplayer.pendingVerification && !b.multiplayer.pendingVerification && samePublishedStep(a, b),
    'Both peers verify Broodscale progression', 30000);
    row.interactions.push({ actor, seq: previousSequence + 1, command, ...submission,
      turn: state.turn_number, priorityRevision: state.priority_revision, actionMenuReadyObservationMs,
      observedSequence: result.host.multiplayer.lastAppliedSequence,
      publicationObservationMs: performance.now() - publicationStart,
      callerThroughVerificationMs: performance.now() - before,
      publicationTargets: [result.host, result.guest].map(view => ({ sequence: view.multiplayer.lastAppliedSequence,
        identity: publicationIdentity(view.state) })) });
    console.log(`[Verified Broodscale seat ${seat}] turn ${state.turn_number} seq ${previousSequence + 1}: ${command.action_ref?.kind || command.type}`);
  }
  assert.fail('Natural Verified Broodscale game exceeded command bound');
}
