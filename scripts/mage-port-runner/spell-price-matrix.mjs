import { assert, captureRuntimeState, releaseRuntimeState, restoreRuntimeState } from "../wasm-test-harness.mjs";
import { registerMageFixtureSource } from "./custom-fixtures.mjs";

const NATIVE_COLORS = {
  White: "W", Blue: "U", Black: "B", Red: "R", Green: "G", Colorless: "C", Snow: "S",
};

function announcedPrice(request) {
  assert(request?.reason === "CastSpell", "expected an authoritative spell payment request", request);
  let generic = 0;
  const colored = [];
  for (const pip of request.cost.pips) {
    assert(pip.length === 1, "expected fixed pips after all hybrid announcements", request.cost);
    const [symbol] = pip;
    if (symbol?.Generic !== undefined) {
      generic += symbol.Generic;
    } else {
      const text = NATIVE_COLORS[symbol];
      assert(text, "unsupported pip in a spell price fixture", pip);
      colored.push(text);
    }
  }
  return [generic, ...colored.sort()].join(",");
}

function collectCastPrices(game, state, name, player, path, prices) {
  const { decision } = state;
  if (decision?.kind === "mana_payment") {
    assert(state.mana_payment?.can_confirm &&
      Number(decision.player) === player, `expected an executable native payment for ${name}`, state.mana_payment);
    const request = JSON.parse(game.exportManaPaymentOptionsRequest(decision.request_hash, decision.plan_id));
    prices.set(path.join(","), announcedPrice(request));
    const confirmed = game.dispatch({ type: "mana_payment", response: {
      action: "confirm", plan_id: decision.plan_id, request_hash: decision.request_hash,
    } });
    assert(confirmed.stack_objects.some(entry => entry.name === name),
      `expected a paid ${name} on the stack`, confirmed.decision);
    return;
  }
  if (decision?.kind === "priority") {
    assert(state.stack_objects.some(entry => entry.name === name),
      `expected ${name} to be cast with no mana payment`, state.stack_objects);
    prices.set(path.join(","), "0");
    return;
  }
  assert(decision?.kind === "select_options" && decision.choice_kind === "hybrid_payment" &&
    decision.min === 1 && decision.max === 1 && Number(decision.player) === player,
    `expected a hybrid announcement or payment for ${name}`, decision);
  const saved = captureRuntimeState(game);
  try {
    for (const option of decision.options.filter(option => option.legal)) {
      const symbol = /^\{([^}]+)\}/.exec(option.description)?.[1];
      assert(symbol, "missing displayed mana symbol on a hybrid announcement", option);
      const result = game.dispatch({ type: "select_options", option_indices: [option.index] });
      collectCastPrices(game, result, name, player, [...path, symbol], prices);
      restoreRuntimeState(game, saved);
    }
  } finally {
    releaseRuntimeState(saved);
  }
}

export function assertNativeSpellPriceMatrix(game, player, cases) {
  const baseline = captureRuntimeState(game);
  try {
    for (const [index, row] of cases.entries()) {
      const name = `Spell Price Fixture ${index + 1}`;
      registerMageFixtureSource(game, { name, typeLine: "Artifact", manaCost: row.printed || "{0}" });
      const spell = Number(game.addCardToZone(player, name, "hand", true));
      if (row.reduction) {
        const amount = Math.abs(row.reduction);
        const direction = row.reduction > 0 ? "less" : "more";
        const modifier = `Cast Price Fixture ${amount} ${direction}`;
        registerMageFixtureSource(game, { name: modifier, typeLine: "Artifact",
          oracleText: `Artifact spells you cast cost {${amount}} ${direction} to cast.` });
        game.addCardToZone(player, modifier, "battlefield", true);
      }
      const state = game.uiState();
      const cast = state.decision?.actions?.find(action => action.action_ref?.kind === "cast_spell" &&
        action.action_ref.casting_method.kind === "normal" && Number(action.action_ref.spell_id) === spell);
      assert(cast, `expected a native cast for ${row.printed || "{0}"} adjusted by ${row.reduction}`);
      const prices = new Map();
      const next = game.dispatch({ type: "priority_action", action_ref: cast.action_ref });
      collectCastPrices(game, next, name, player, [], prices);
      const actual = [...prices].sort();
      const expected = Object.entries(row.payments).sort();
      assert(JSON.stringify(actual) === JSON.stringify(expected),
        `native cast prices differ for ${row.printed || "{0}"} adjusted by ${row.reduction}`, { actual, expected });
      restoreRuntimeState(game, baseline);
    }
  } finally {
    restoreRuntimeState(game, baseline);
    releaseRuntimeState(baseline);
  }
}
