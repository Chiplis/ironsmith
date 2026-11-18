import {
  assert,
  captureRuntimeState,
  releaseRuntimeState,
  restoreRuntimeState,
  selectObjectsCommand,
} from "../wasm-test-harness.mjs";

const TYPES = ["W", "U", "B", "R", "G", "C"];
const POOL_FIELDS = ["white", "blue", "black", "red", "green", "colorless"];
const MANA_SYMBOL_CODES = new Map(POOL_FIELDS.map((field, index) =>
  [field[0].toUpperCase() + field.slice(1), TYPES[index]]));
const MAX_PROBE_BRANCHES = 6144;

function poolCounts(player, auditRestrictions = false) {
  assert(player?.manaPool, "unsupported mana-option assertion (native mana pool required)");
  assert(auditRestrictions || !player.restrictedMana?.length,
    "unsupported mana-option assertion for mana with spending restrictions", player.restrictedMana);
  const counts = POOL_FIELDS.map(type => player.manaPool[type]);
  assert(counts.every(value => Number.isSafeInteger(value) && value >= 0),
    "unsupported mana-option assertion (invalid or oversized native mana pool)", player.manaPool);
  return counts;
}

function matchesColoredSpellWithoutX(cast) {
  if (!cast?.restrict_to_matching_spell || cast.grant_uncounterable ||
      cast.enters_with_counters.length || cast.granted_abilities.length) return false;
  const {
    colors, no_x_in_cost: withoutX, union_surface: _surface,
    power_reference: powerReference, toughness_reference: toughnessReference, ...otherPredicates
  } = cast.filter;
  return colors === 31 && withoutX &&
    powerReference === "Effective" && toughnessReference === "Effective" &&
    Object.values(otherPredicates).every(value => !value || (Array.isArray(value) && value.length === 0));
}

function restrictionProfile(units) {
  const quantities = {};
  for (const unit of units) {
    const [rule] = unit.restrictions;
    const symbol = MANA_SYMBOL_CODES.get(unit.symbol);
    assert(symbol && unit.restrictions.length === 1,
      "unsupported mana-option restriction shape", unit);
    const transaction = rule?.PaymentTransaction;
    const cast = rule?.CastSpellMatching;
    const simpleTypeCast = rule?.CastSpell;
    let name;
    if (simpleTypeCast) {
      const { card_types: types, subtype_requirement: subtype, restrict_to_matching_spell: restricted,
        grant_uncounterable: uncounterable, enters_with_counters: counters, granted_abilities: abilities,
        ...additionalPredicates } = simpleTypeCast;
      if (restricted && types.length === 1 && subtype == null && !uncounterable && !counters.length && !abilities.length &&
          Object.keys(additionalPredicates).length === 0) {
        name = `${symbol}:cast:${types[0]}`;
      }
    } else if (symbol === "C" && rule === "ActivateAbility") {
      name = "activateAbilities";
    } else if (symbol === "C" && transaction?.restriction === "CostContainsX" && transaction.on_spend.length === 0) {
      name = "costContainsX";
    } else if (symbol === "C" && matchesColoredSpellWithoutX(cast)) {
      name = "coloredSpellWithoutX";
    }
    assert(name, "unsupported mana-option restriction predicate", unit);
    quantities[name] = (quantities[name] ?? 0) + 1;
  }
  return quantities;
}

function restrictionKey(restricted) {
  return Object.entries(restricted).sort(([left], [right]) => left.localeCompare(right));
}

function poolKey({ counts, restricted }) {
  return JSON.stringify([counts, restrictionKey(restricted)]);
}

function* expectedPools(pattern, pool = Array(TYPES.length).fill(0)) {
  if (!pattern) {
    yield pool;
    return;
  }
  const match = /^\{([WUBRGC]|Any)\}/.exec(pattern);
  assert(match, `unsupported mana-option notation: ${pattern}`);
  const rest = pattern.slice(match[0].length);
  const choices = match[1] === "Any" ? [0, 1, 2, 3, 4] : [TYPES.indexOf(match[1])];
  for (const choice of choices) {
    const next = [...pool];
    next[choice]++;
    yield* expectedPools(rest, next);
  }
}

function* decisionCommands(decision) {
  const { min, max } = decision;
  if (decision.kind === "number") {
    assert(Number.isSafeInteger(min) && Number.isSafeInteger(max) && min <= max &&
      max - min < MAX_PROBE_BRANCHES, "unsupported mana-option number range", decision);
    for (let value = min; value <= max; value++) yield { type: "number_choice", value };
    return;
  }
  const options = decision.kind === "select_options";
  const objects = decision.kind === "select_objects";
  assert(options || objects, `unsupported mana-option decision ${decision.kind}`, decision);
  const repeatedColors = options && decision.choice_kind === "colors" &&
    decision.options.every(option => option.repeatable && option.point_cost === 1);
  assert(Number.isInteger(min) && Number.isInteger(max) && min >= 0 && min <= max &&
    (max <= 1 || (repeatedColors && max <= 64)),
    "unsupported mana-option decision with multiple selections", decision);
  const candidates = (options ? decision.options : decision.candidates).filter(candidate => candidate.legal !== false);
  const cmd = ids => options ? { type: "select_options", option_indices: ids }
    : selectObjectsCommand(decision, ids);
  function* choose(from, selected) {
    if (selected.length >= min) yield cmd(selected);
    if (selected.length === max) return;
    for (let index = from; index < candidates.length; index++) {
      const candidate = candidates[index];
      const id = options ? candidate.index : candidate.id;
      const used = selected.filter(value => value === id).length;
      if (!used || used < candidate.max_count) {
        yield* choose(repeatedColors ? index : index + 1, [...selected, id]);
      }
    }
  }
  let found = false;
  for (const command of choose(0, [])) {
    found = true;
    yield command;
  }
  assert(found, "mana-option probe found no legal decision answers", decision);
}

function* actualAllocations(amounts, total, allocation = []) {
  if (allocation.length === amounts.length) {
    if (total === 0) yield allocation;
    return;
  }
  const available = amounts[allocation.length];
  const remaining = amounts.slice(allocation.length + 1).reduce((sum, count) => sum + count, 0);
  for (let count = Math.max(0, total - remaining); count <= Math.min(available, total); count++) {
    yield* actualAllocations(amounts, total - count, [...allocation, count]);
  }
}

function* paymentCommands(game, state, prefundManaCosts) {
  const payment = state.mana_payment;
  const decision = state.decision;
  const request = JSON.parse(game.exportManaPaymentOptionsRequest(decision.request_hash, decision.plan_id));
  const exact = request?.preferences?.actual_mana_allocation;
  if (exact && payment?.planning_complete && !payment.can_confirm) return;
  assert(request?.reason === "ActivateManaAbility" && payment?.planning_complete && payment.can_confirm,
    "unsupported mana-option payment without a complete native plan", payment);
  if (payment.planned_sources.length) {
    // Some fixtures allow all required funding abilities at ordinary priority,
    // before the paid ability locks its costs. Those activations have their
    // own branches in the search.
    const tapReservationsOnly = payment.reserved_sources.every(source =>
      payment.fixed_excluded_source_ids.includes(source.source_id));
    assert(prefundManaCosts && tapReservationsOnly,
      "unsupported mana-option payment that requires concurrent source activations", payment);
    return;
  }
  const amounts = poolCounts({ manaPool: payment.pool_before });
  const remaining = poolCounts({ manaPool: payment.pool_after_payment });
  const paysWholePool = remaining.every(count => count === 0);
  const fixedSymbols = payment.pips.every(pip => pip.length === 1 && TYPES.includes(pip[0]));
  const confirm = { type: "mana_payment", response: {
    action: "confirm", plan_id: decision.plan_id, request_hash: decision.request_hash,
  } };
  assert(payment.life_to_pay === 0 && payment.allocations.every(choice => choice.payment_kind === "mana"),
    "unsupported mana-option payment with nonmana allocations", payment);
  if (exact) {
    assert(exact.every((count, index) => count === amounts[index] - remaining[index]),
      "mana-option probe native plan spent a different mana allocation", payment);
  } else if (amounts.filter(Boolean).length > 1 && !paysWholePool && !fixedSymbols) {
    assert(payment.pips.every(pip => /^\d+$/.test(pip)),
      "unsupported mana-option payment with hybrid or mixed pips", payment);
    const total = payment.allocations.length;
    const planned = amounts.map((count, index) => count - remaining[index]);
    for (const allocation of actualAllocations(amounts, total)) {
      // The current native plan is already a proof of this exact allocation.
      yield allocation.every((count, index) => count === planned[index]) ? confirm :
        { type: "mana_payment", response: { action: "replan", actual_mana_allocation: allocation } };
    }
    return;
  }
  yield confirm;
}

function formatPool({ counts, restricted }) {
  const mana = counts.map((count, index) => `{${TYPES[index]}}`.repeat(count)).join("") || "{}";
  return Object.keys(restricted).length ? { mana, restricted } : mana;
}

function dominates(larger, smaller) {
  if (smaller.counts.every(count => count === 0)) {
    return larger.counts.some(count => count > 0);
  }
  const smallerKinds = Object.keys(smaller.restricted);
  // Report the maximum for each available combination of spending restrictions.
  if (Object.keys(larger.restricted).length !== smallerKinds.length ||
      smallerKinds.some(name => !Object.hasOwn(larger.restricted, name))) {
    return false;
  }
  const unrestrictedCounts = pool => {
    const counts = [...pool.counts];
    for (const [restriction, count] of Object.entries(pool.restricted)) {
      const symbol = restriction.includes(":") ? restriction.split(":")[0] : "C";
      counts[TYPES.indexOf(symbol)] -= count;
    }
    return counts;
  };
  const largerUnrestricted = unrestrictedCounts(larger);
  const smallerUnrestricted = unrestrictedCounts(smaller);
  return largerUnrestricted.every((count, index) => count >= smallerUnrestricted[index]) &&
    smallerKinds.every(name => larger.restricted[name] >= smaller.restricted[name]) &&
    (largerUnrestricted.some((count, index) => count > smallerUnrestricted[index]) ||
      smallerKinds.some(name => larger.restricted[name] > smaller.restricted[name]));
}

export function assertNativeManaOptions(game, player, operation) {
  const { turn_number: turn, phase, step } = game.uiState();
  const visited = new Set();
  const pools = new Map();
  const auditRestrictions = operation.options.some(option => typeof option === "object");
  const branchLimit = operation.branchLimit ?? MAX_PROBE_BRANCHES;
  let branchCount = 0;

  function follow(command) {
    assert(++branchCount <= branchLimit, "mana-option probe branch limit reached before completing the search");
    if (process.env.MAGE_PORT_TRACE_MANA === "1" && branchCount % 2048 === 0) {
      console.error(`[XMage mana probe] ${branchCount} commands, ${visited.size} states, ${pools.size} pools`);
    }
    const state = game.dispatch(command);
    const allocation = command.response?.actual_mana_allocation;
    if (allocation) {
      const decision = state.decision;
      assert(decision?.kind === "mana_payment", "mana-option probe lost the replanned payment decision");
      const request = JSON.parse(game.exportManaPaymentOptionsRequest(decision.request_hash, decision.plan_id));
      assert(JSON.stringify(request?.preferences?.actual_mana_allocation) === JSON.stringify(allocation),
        "unsupported mana-option payment without native exact-spend replanning");
    }
    visit(state);
  }

  function branch(commands) {
    const choices = commands[Symbol.iterator]();
    let next = choices.next();
    if (next.done) return;
    let following = choices.next();
    if (following.done) {
      follow(next.value);
      return;
    }
    const saved = captureRuntimeState(game);
    try {
      while (!following.done) {
        follow(next.value);
        restoreRuntimeState(game, saved);
        next = following;
        following = choices.next();
      }
    } finally {
      releaseRuntimeState(saved);
    }
    follow(next.value);
  }

  function visit(state) {
    assert(state.turn_number === turn && state.phase === phase && state.step === step,
      "mana-option probe advanced past the authored checkpoint", state.decision);
    const decision = state.decision;
    assert(decision, "mana-option probe lost its decision before completing the search");
    if (decision.kind !== "priority") {
      branch(decision.kind === "mana_payment"
        ? paymentCommands(game, state, operation.prefundManaCosts) : decisionCommands(decision));
      return;
    }
    assert(Number(state.priority_player) === player, "mana-option probe lost the queried player's priority");
    assert(decision.analysis_complete, "unsupported mana-option assertion with incomplete legal-action analysis");
    const audit = game.exportPublicAuditCheckpoint();
    const seat = audit.players.find(entry => Number(entry.id) === player);
    const pool = {
      counts: poolCounts(seat, auditRestrictions),
      restricted: auditRestrictions ? restrictionProfile(seat.restrictedMana ?? []) : {},
    };
    pools.set(poolKey(pool), pool);
    // Dedupe only states with identical native audit data and mana actions.
    let actions = decision.actions.filter(action =>
      action.action_ref?.kind === "activate_mana_ability" && action.payment_proven === true);
    if (operation.interchangeableSources?.length) {
      const inventory = new Map(audit.objects.map(object => [Number(object.id), object.identity?.name]));
      const unique = new Set();
      // The authored fixture opts in only for copies with no effects that
      // distinguish their individual identities.
      actions = actions.filter(({ action_ref: action }) => {
        const name = inventory.get(Number(action.source));
        if (!operation.interchangeableSources.includes(name)) return true;
        const key = `${name}:${action.ability_index}`;
        if (unique.has(key)) return false;
        unique.add(key);
        return true;
      });
    }
    const comparable = operation.ignoreGraveyardHistory ? {
      ...audit,
      // Opt in only when no effect or mana action uses graveyard cards or
      // their timestamps. Sacrificed copies then differ only in incarnation.
      players: audit.players.map(player => ({ ...player, graveyard: player.graveyard.length })),
      objects: audit.objects.filter(object => object.zone !== "graveyard"),
    } : audit;
    const key = JSON.stringify([comparable, actions.map(action => action.action_ref)]);
    if (visited.has(key)) return;
    visited.add(key);
    branch(actions.map(action => ({ type: "priority_action", action_ref: action.action_ref })));
  }

  visit(game.uiState());
  const expected = new Map(operation.options.flatMap(option =>
    Array.from(expectedPools(option.mana ?? option), counts => {
      const pool = { counts, restricted: option.restricted ?? {} };
      return [poolKey(pool), pool];
    })));
  const reachable = [...pools.values()];
  const missing = [...expected].filter(([key]) => !pools.has(key)).map(([, counts]) => formatPool(counts));
  const missingReachable = (operation.reachableOptions ?? []).flatMap(option =>
    Array.from(expectedPools(option), counts => ({ counts, restricted: {} }))
      .filter(pool => !pools.has(poolKey(pool))).map(formatPool));
  // Most XMage checks list maximal pools, while some require every reachable alternative.
  const maximalOnly = operation.exact !== "all_reachable";
  const unexpected = operation.exact ? reachable
    .filter(pool => !expected.has(poolKey(pool)) && (!maximalOnly || !reachable.some(other => dominates(other, pool))))
    .map(formatPool) : [];
  const dominated = operation.exact && maximalOnly ? [...expected.values()]
    .filter(pool => reachable.some(other => dominates(other, pool))).map(formatPool) : [];
  assert(!missing.length && !missingReachable.length && !unexpected.length && !dominated.length,
    `native mana options differ for player ${player}`, { missing, missingReachable, unexpected, dominated, branchCount });
}
