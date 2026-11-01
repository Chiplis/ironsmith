import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

const bolt = { id: 7, name: "Lightning Bolt", card_types: ["instant"], type_line: "Instant" };
const bears = { id: 8, name: "Grizzly Bears", card_types: ["creature"], type_line: "Creature — Bear" };
const ritual = { id: 9, name: "Dark Ritual", card_types: ["instant"], type_line: "Instant" };

const action = (overrides) => ({
  kind: "cast_spell",
  drag_requires_targets: false,
  drag_requires_modes: false,
  ...overrides,
  action_ref: { kind: "cast_spell", spell_id: overrides.object_id, ...(overrides.action_ref || {}) },
});

const state = {
  perspective: 0,
  players: [{ id: 0, index: 0, name: "You", can_view_hand: true, hand_cards: [bolt, bears, ritual] }],
  decision: {
    kind: "priority",
    player: 0,
    actions: [
      action({ index: 0, object_id: 7, label: "Cast Lightning Bolt", drag_requires_targets: true }),
      action({ index: 1, object_id: 8, label: "Cast Grizzly Bears" }),
      action({ index: 2, object_id: 9, label: "Cast Dark Ritual" }),
      action({ index: 3, object_id: 9, label: "Cast Dark Ritual with kicker" }),
    ],
  },
};

async function harness(fixtureState = state, mainAction = false) {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0, hmr: false }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1200, height: 700 }, reducedMotion: "reduce" });
  const errors = [];
  page.on("pageerror", (error) => errors.push(String(error?.message || error)));
  await page.route("**/api.scryfall.com/**", (route) => route.abort());
  await page.route("**/cards.scryfall.io/**", (route) => route.abort());
  await page.addInitScript((fixture) => { window.__handKeyboardFixture = fixture; localStorage.setItem('ironsmith.priorityShortcutPreset','classic'); }, { state: fixtureState, mainAction });
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/hand-keyboard-cast.html`, { waitUntil: "domcontentloaded" });
  await page.locator('[data-hand-case] .game-card.hand-card').first().waitFor();
  return {
    page,
    errors,
    casts: () => page.evaluate(() => window.__keyboardCasts),
    drag: () => page.evaluate(() => window.__dragState),
    press: async (objectId) => {
      await page.locator(`[data-hand-case] .game-card[data-object-id="${objectId}"]`).focus();
      await page.keyboard.press(" ");
      await page.waitForTimeout(120);
    },
    close: async () => { await browser.close(); await vite.close(); },
  };
}

test("number shortcuts inspect multi-digit hand positions without casting", { timeout: 120000 }, async () => {
  const large = structuredClone(state);
  large.players[0].hand_cards = Array.from({ length: 12 }, (_, i) => ({ ...bears, id: 100 + i }));
  large.decision.actions = [];
  const { page, casts, errors, close } = await harness(large);
  const selected = () => page.evaluate(() => document.activeElement?.getAttribute('data-object-id'));
  try {
    await page.keyboard.press('1');
    assert.equal(await selected(), '100');
    await page.keyboard.press('1');
    assert.equal(await selected(), '110');
    await page.keyboard.press('Escape');
    await page.keyboard.press('1');
    await page.keyboard.press('0');
    assert.equal(await selected(), '109');
    await page.keyboard.press('Escape');
    await page.keyboard.press('1');
    await page.waitForTimeout(750);
    await page.keyboard.press('1');
    assert.equal(await selected(), '100', 'expired prefix starts a new number');
    await page.keyboard.press('9');
    assert.equal(await selected(), '108', 'out-of-range 19 starts a new number');
    await page.keyboard.press('Control+2');
    assert.equal(await selected(), '108');
    await page.evaluate(() => {
      const input = document.createElement('input');
      input.id = 'typing-test';
      document.body.append(input);
      input.focus();
    });
    await page.keyboard.type('11');
    assert.equal(await page.locator('#typing-test').inputValue(), '11');
    assert.equal(await selected(), null);
    await page.evaluate(() => {
      document.activeElement.blur();
      const dialog = document.createElement('dialog');
      document.body.append(dialog);
      dialog.showModal();
    });
    await page.keyboard.press('1');
    assert.equal(await selected(), null, 'dialogs keep number keys');
    assert.deepEqual(await casts(), []);
    assert.deepEqual(errors, []);
  } finally { await close(); }
});

test("small hands select single digits immediately", { timeout: 120000 }, async () => {
  const { page, casts, close } = await harness();
  try {
    await page.keyboard.press('1');
    await page.keyboard.press('2');
    assert.equal(await page.evaluate(() => document.activeElement.getAttribute('data-object-id')), '8');
    assert.deepEqual(await casts(), []);
  } finally { await close(); }
});

test("H and period interchangeably toggle the numbered hand", { timeout: 120000 }, async () => {
  const large = structuredClone(state);
  large.players[0].hand_cards = Array.from({ length: 12 }, (_, i) => ({ ...bears, id: 100 + i }));
  const { page, casts, errors, close } = await harness(large);
  try {
    assert.equal(await page.locator('.hand-card-number').count(), 0);
    await page.keyboard.press('h');
    assert.equal(await page.locator('[data-hand-case]').getAttribute('data-hand-open'), 'true');
    assert.deepEqual(await page.locator('.hand-card-number').allTextContents(), Array.from({ length: 12 }, (_, i) => String(i + 1)));
    assert.equal(await page.locator('.hand-card-number').first().isVisible(), true);
    await page.keyboard.press('1');
    await page.keyboard.press('1');
    assert.equal(await page.evaluate(() => document.activeElement.getAttribute('data-object-id')), '110');
    assert.equal(await page.locator('.hand-card[data-object-id="110"] .hand-card-number').count(), 0, 'selected card name stays unobstructed');
    assert.equal(await page.locator('.hand-card[data-object-id="100"] .hand-card-number').textContent(), '1');
    await page.keyboard.press('ArrowLeft');
    assert.equal(await page.locator('.hand-card[data-object-id="109"] .hand-card-number').count(), 0);
    assert.equal(await page.locator('.hand-card[data-object-id="110"] .hand-card-number').textContent(), '11', 'previous card regains its position badge');
    await page.keyboard.press('.');
    assert.equal(await page.locator('[data-hand-case]').getAttribute('data-hand-open'), 'false');
    assert.equal(await page.locator('.hand-card-number').count(), 0);
    for (const key of ['.', '.', 'h', 'h', '.', 'h', 'h', '.']) {
      const wasOpen = await page.locator('[data-hand-case]').getAttribute('data-hand-open') === 'true';
      await page.keyboard.press(key);
      assert.equal(await page.locator('[data-hand-case]').getAttribute('data-hand-open'), String(!wasOpen));
      assert.equal(await page.locator('.hand-card-number').count(), wasOpen ? 0 : 12);
    }
    await page.keyboard.press('Control+h');
    assert.equal(await page.locator('[data-hand-case]').getAttribute('data-hand-open'), 'false', 'modified browser shortcuts do not toggle the hand');
    await page.evaluate(() => {
      const input = document.createElement('input');
      document.body.append(input);
      input.focus();
    });
    await page.keyboard.press('h');
    assert.equal(await page.locator('[data-hand-case]').getAttribute('data-hand-open'), 'false');
    assert.deepEqual(await casts(), []);
    assert.deepEqual(errors, []);
  } finally { await close(); }
});

test("timing-legal hand cards are white and clickable until payment is proven", async () => {
  const pending = structuredClone(state);
  pending.decision.actions[0].payment_proven = false;
  const { page, press, casts, close } = await harness(pending);
  try {
    const borderColor = id => page.locator(`.hand-card[data-object-id="${id}"] .card-action-border`)
      .evaluate(border => getComputedStyle(border).borderTopColor);
    assert.equal(await borderColor(7), "rgb(255, 255, 255)");
    assert.equal(await borderColor(8), "rgb(255, 224, 131)");
    await press(7);
    assert.deepEqual((await casts())[0].actions, ["Cast Lightning Bolt"]);
  } finally { await close(); }
});

test("the activation key casts a hand card without waiting for a pointer release", { timeout: 120000 }, async () => {
  const { press, casts, drag, errors, close } = await harness();
  try {
    // A targeted spell is cast at once; the engine's targeting arrow, not a
    // held card, is what the mouse then aims.
    await press(7);
    assert.deepEqual(await drag(), null, "nothing is left held");
    let requested = await casts();
    assert.equal(requested.length, 1);
    assert.deepEqual(requested[0].actions, ["Cast Lightning Bolt"]);
    assert.equal(requested[0].objectId, 7);
    assert.ok(requested[0].anchorRect.width > 0, "the picker can be anchored on the card");

    // Several ways to play one card are published as one request, so the
    // picker resolves the choice before anything reaches the engine.
    await press(9);
    requested = await casts();
    assert.equal(requested.length, 2);
    assert.deepEqual(requested[1].actions, ["Cast Dark Ritual", "Cast Dark Ritual with kicker"]);
    assert.deepEqual(await drag(), null);
  } finally {
    assert.deepEqual(errors, []);
    await close();
  }
});

test("a permanent stays held so its battlefield slot follows the mouse", { timeout: 120000 }, async () => {
  const { page, press, casts, drag, errors, close } = await harness();
  try {
    const cardBox = await page.locator('[data-hand-case] .game-card[data-object-id="8"]').boundingBox();
    await press(8);
    assert.deepEqual(await casts(), [], "a permanent is not cast until it is placed");
    const held = await drag();
    assert.equal(held.objectId, 8);
    assert.equal(held.keyboard, true);
    assert.deepEqual(held.actions, ["Cast Grizzly Bears"]);
    const anchor = { x: held.currentX, y: held.currentY };
    const grip = { x: held.startX, y: held.startY };

    // The arrow stands in for the pointer from the moment the card is held,
    // pointing at dead space above the card rather than at the resting mouse.
    await page.locator(".placement-drag-arrow").waitFor({ timeout: 5000 });
    assert.ok(anchor.y < cardBox.y, "lifted clear of the card it came from");
    assert.ok(Math.abs(anchor.x - (cardBox.x + (cardBox.width / 2))) < 1, "straight ahead of it");
    const overCard = await page.evaluate(
      ([x, y]) => Boolean(document.elementFromPoint(x, y)?.closest?.(".game-card")),
      [anchor.x, anchor.y],
    );
    assert.equal(overCard, false, "nothing a click would pick sits under the arrow");

    // The hold tracks the bare pointer: no button is down to drag with.
    const sessionUpdates = await page.evaluate(() => window.__dragSessionUpdates);
    await page.mouse.move(880, 220);
    await page.waitForTimeout(80);
    let moved = await drag();
    assert.deepEqual([moved.currentX, moved.currentY], [880, 220]);
    assert.equal(await page.evaluate(() => window.__dragSessionUpdates), sessionUpdates, "pointer motion does not republish the hand/workspace session");
    assert.notDeepEqual([moved.currentX, moved.currentY], [anchor.x, anchor.y], "the mouse takes the arrow over");
    assert.deepEqual([moved.startX, moved.startY], [grip.x, grip.y], "it still points from the card");

    await page.mouse.move(300, 480);
    await page.waitForTimeout(80);
    moved = await drag();
    assert.deepEqual([moved.currentX, moved.currentY], [300, 480]);

    // Escape puts the card back; with no button to lift nothing else would.
    await page.keyboard.press("Escape");
    await page.waitForTimeout(80);
    assert.deepEqual(await drag(), null);
    await page.mouse.move(500, 300);
    await page.waitForTimeout(80);
    assert.deepEqual(await drag(), null, "the released hold stops tracking");
  } finally {
    assert.deepEqual(errors, []);
    await close();
  }
});


test("an accepted drop tucks the hand and suppresses hover for 200ms", async () => {
  const h = await harness();
  try {
    const card = h.page.locator('[data-hand-case] .game-card[data-object-id="8"]');
    await card.focus();
    await h.page.evaluate(() => {
      window.dispatchEvent(new CustomEvent("ironsmith:hand-card-dropped"));
      const card = document.querySelector('[data-hand-case] .game-card[data-object-id="8"]');
      card.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
    });
    await h.page.waitForTimeout(80);
    assert.equal(await card.evaluate((el) => getComputedStyle(el).pointerEvents), "none");
    assert.equal(await card.evaluate((el) => el.classList.contains("inspected") || el.classList.contains("hovered")), false);
    await h.page.waitForFunction(() => !document.documentElement.hasAttribute("data-hand-drop-cooldown"));
    assert.notEqual(await card.evaluate((el) => getComputedStyle(el).pointerEvents), "none");
    await card.hover();
    await h.page.waitForFunction(() => document.querySelector('[data-hand-case] .game-card[data-object-id="8"]').classList.contains("hovered"));
    assert.deepEqual(h.errors, []);
  } finally {
    await h.close();
  }
});

test("Enter on a keyboard-selected card puts the hand back down", { timeout: 120000 }, async () => {
  const { page, casts, errors, close } = await harness();
  const handState = () => page.evaluate(() => ({
    open: document.querySelector("[data-hand-case]")?.getAttribute("data-hand-open"),
    keyboardNavigation: Boolean(document.querySelector('[data-hand-case] [data-keyboard-navigation="true"]')),
    focusedHandCard: Boolean(document.activeElement?.closest?.("[data-hand-case] .game-card")),
  }));
  try {
    // Pick the card by number, walk to it with the arrows, then confirm.
    for (const route of [["3"], ["1", "ArrowRight", "ArrowRight"]]) {
      for (const key of route) await page.keyboard.press(key);
      await page.waitForTimeout(120);
      const before = await handState();
      assert.equal(before.focusedHandCard, true, `${route.join(" ")} selects a hand card`);
      const requested = (await casts()).length;
      await page.keyboard.press("Enter");
      await page.waitForTimeout(150);
      assert.equal((await casts()).length, requested + 1, "the several ways to play it go to the picker");
      assert.deepEqual(await handState(), { open: "false", keyboardNavigation: false, focusedHandCard: false },
        `after ${route.join(" ")} then Enter the hand returns to rest`);
    }
  } finally {
    assert.deepEqual(errors, []);
    await close();
  }
});

test("MTGO numbers select the open hand and resume priority shortcuts when closed", { timeout: 120000 }, async () => {
  const large = structuredClone(state);
  large.players[0].hand_cards = Array.from({ length: 12 }, (_, i) => ({ ...bears, id: 100 + i }));
  large.decision.actions = [];
  const { page, casts, errors, close } = await harness(large);
  const selected = () => page.evaluate(() => document.activeElement?.getAttribute('data-object-id'));
  try {
    await page.evaluate(async () => {
      localStorage.setItem('ironsmith.priorityShortcutPreset', 'mtgo');
      const { installPriorityShortcuts } = await import('/src/lib/priority-shortcuts.js');
      window.__priorityKeys = [];
      installPriorityShortcuts(document, {
        state: () => window.__handKeyboardFixture.state,
        start: mode => window.__priorityKeys.push(mode),
        clear: () => window.__priorityKeys.push('clear'),
        group: () => window.__priorityKeys.push('group'),
        maintain: () => {}, answer: () => window.__priorityKeys.push('answer'),
      });
    });
    await page.keyboard.press('h');
    await page.keyboard.press('1');
    await page.keyboard.press('1');
    assert.equal(await selected(), '110');
    await page.keyboard.press('6');
    assert.equal(await selected(), '105');
    assert.deepEqual(await page.evaluate(() => window.__priorityKeys), []);
    await page.keyboard.press('.');
    await page.keyboard.press('6');
    assert.deepEqual(await page.evaluate(() => window.__priorityKeys), ['turn']);
    await page.keyboard.press('.');
    await page.keyboard.press('2');
    assert.equal(await selected(), '101');
    assert.deepEqual(await page.evaluate(() => window.__priorityKeys), ['turn']);
    assert.deepEqual(await casts(), []);
    assert.deepEqual(errors, []);
  } finally { await close(); }
});


test("open hand captures all arrows from other zones and wraps within its own cards", { timeout: 60000 }, async () => {
  const { page, casts, errors, close } = await harness();
  const selected = () => page.evaluate(() => document.activeElement?.getAttribute('data-object-id'));
  try {
    await page.evaluate(() => {
      window.__fieldArrows = 0;
      const field = document.createElement('button');
      field.id = 'field-card'; field.className = 'game-card field-card';
      field.setAttribute('role', 'button'); field.setAttribute('data-object-id', 'field');
      field.textContent = 'Field card';
      field.onkeydown = event => { if (event.key.startsWith('Arrow')) window.__fieldArrows++; };
      document.body.append(field); field.focus();
    });
    await page.keyboard.press('.');
    for (const [key, expected] of [['ArrowRight', '7'], ['ArrowDown', '8'], ['ArrowUp', '7'], ['ArrowLeft', '9'], ['ArrowRight', '7']]) {
      await page.keyboard.press(key);
      assert.equal(await selected(), expected);
    }
    // Even if focus returns to a card in another zone, open-hand navigation wins.
    await page.locator('#field-card').focus();
    await page.keyboard.press('ArrowUp');
    assert.equal(await selected(), '9');
    assert.equal(await page.evaluate(() => window.__fieldArrows), 0);
    await page.evaluate(() => {
      const input = document.createElement('input'); input.id = 'arrow-input';
      input.value = 'abc'; document.body.append(input); input.focus(); input.setSelectionRange(1, 1);
    });
    await page.keyboard.press('ArrowRight');
    assert.equal(await page.locator('#arrow-input').evaluate(input => input.selectionStart), 2);
    await page.evaluate(() => {
      const dialog = document.createElement('dialog'); dialog.id = 'arrow-dialog';
      const button = document.createElement('button'); button.textContent = 'Dialog control';
      dialog.append(button); document.body.append(dialog); dialog.showModal(); button.focus();
    });
    await page.keyboard.press('ArrowLeft');
    assert.equal(await page.evaluate(() => document.activeElement.closest('dialog')?.id), 'arrow-dialog');
    await page.evaluate(() => document.querySelector('#arrow-dialog').remove());
    await page.locator('.hand-card[data-object-id="9"]').focus();
    await page.keyboard.press('h');
    assert.equal(await selected(), null, 'closing clears hand focus');
    await page.locator('#field-card').focus();
    await page.keyboard.press('ArrowRight');
    assert.equal(await page.evaluate(() => window.__fieldArrows), 1, 'closed hand releases navigation');
    assert.deepEqual(await casts(), []);
    assert.deepEqual(errors, []);
  } finally { await close(); }
});


test("Space uses the hand action and its label follows payment analysis", { timeout: 60000 }, async () => {
  const fixture = structuredClone(state);
  fixture.decision.analysis_complete = false;
  fixture.decision.actions = [
    { kind: 'pass_priority', index: 0, label: 'Pass priority' },
    action({ index: 1, object_id: 7, label: 'Cast Lightning Bolt', payment_proven: false }),
    { kind: 'activate_ability', index: 2, object_id: 7, label: 'Activate hand ability', payment_proven: false,
      action_ref: { kind: 'activate_ability', source_id: 7, ability_index: 0 } },
  ];
  const { page, casts, errors, close } = await harness(fixture, true);
  const update = async (play, activate, complete = true) => {
    await page.evaluate(({ play, activate, complete }) => window.__setHandFixtureState(previous => ({
      ...previous, decision: { ...previous.decision, analysis_complete: complete,
        actions: previous.decision.actions.map(action => ({ ...action,
          ...(action.kind === 'cast_spell' ? { payment_proven: play }
            : action.kind === 'activate_ability' ? { payment_proven: activate } : {}) })) },
    })), { play, activate, complete });
  };
  try {
    await page.keyboard.press('h');
    await page.keyboard.press('Space');
    assert.equal(await page.evaluate(() => Boolean(window.__passed)), false, 'no selection must not pass');
    await page.keyboard.press('ArrowRight');
    const both = page.getByRole('button', { name: 'Play/Activate', exact: true });
    await both.waitFor();
    assert.equal(await both.locator('kbd').textContent(), 'Space');
    assert.equal(await page.getByRole('button', { name: 'Pass priority', exact: true }).count(), 0);
    await update(true, false);
    await page.getByRole('button', { name: 'Play', exact: true }).waitFor();
    await update(false, true);
    await page.getByRole('button', { name: 'Activate', exact: true }).waitFor();
    await page.keyboard.press('Space');
    await page.waitForFunction(() => window.__keyboardCasts.length === 1);
    assert.deepEqual((await casts())[0].actions, ['Activate hand ability']);
    assert.equal(await page.evaluate(() => Boolean(window.__passed)), false);
    await page.getByRole('button', { name: 'Pass priority', exact: true }).waitFor();
    await page.keyboard.press('.');
    await page.keyboard.press('ArrowRight');
    await update(false, false);
    assert.equal(await page.getByRole('button', { name: 'No available action', exact: true }).isDisabled(), true);
    await page.keyboard.press('Space');
    assert.equal((await casts()).length, 1);
    await update(false, false, false);
    await page.getByRole('button', { name: 'Play/Activate', exact: true }).click();
    await page.waitForFunction(() => window.__keyboardCasts.length === 2, null, { timeout: 5000 });
    assert.deepEqual((await casts())[1].actions, ['Cast Lightning Bolt', 'Activate hand ability']);
    assert.deepEqual(errors, []);
  } finally { await close(); }
});
