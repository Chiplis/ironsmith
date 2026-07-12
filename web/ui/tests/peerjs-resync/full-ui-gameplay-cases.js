import {
  UI_ROOT,
  activateButtonNode,
  activateLocalButton,
  assert,
  assertNoFullUiSyncFailures,
  assertNoFullUiSyncFailuresWithDebug,
  assertNoPageErrors,
  assertNoSyncFailureText,
  buttonDebugText,
  captureFullUiStep,
  clickEnabledButton,
  clickLocalButton,
  clickLocalDecisionButton,
  clickUnselectedEnabledButton,
  chromium,
  closePeerServer,
  deckUrlParam,
  driveDemonicConsultationCast,
  freePort,
  fullUiPageDebug,
  fullUiSnapshot,
  fs,
  hasLocalButton,
  openFullUiPage,
  path,
  resolveDemonicConsultationWithMissingName,
  sleep,
  startFullUiPeerMatch,
  startHarnessServer,
  startPeerServer,
  submitFullUiMultiplayerCommand,
  test,
  visibleBodyText,
  visibleHandCardNames,
  waitAndClickLocalButton,
  waitForFullUiPair,
  waitForFullUiSequenceAdvance,
  waitForFullUiSync,
  waitForNamedVisibleHand,
  waitForVisibleBodyText,
  withRejectingTimeout,
  withTimeout,
} from "../peerjs-resync-harness.js";

test("full UI PeerJS casting Demonic Consultation after playing Swamp keeps hidden openings synced", { timeout: 300000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const match = await startFullUiPeerMatch({
      baseUrl,
      hostContext,
      guestContext,
      hostDeckText: "30 Swamp\n30 Demonic Consultation",
      guestDeckText: "60 Lightning Bolt",
      hostName: "Alice P1",
      guestName: "Alice P2",
      hostLabel: "host-consultation-ui",
      guestLabel: "guest-consultation-ui",
    });
    hostPage = match.hostPage;
    guestPage = match.guestPage;

    await assertNoFullUiSyncFailures(hostPage, guestPage);
    await waitAndClickLocalButton(hostPage, "host-keep-consultation-test", /KEEP HAND/i, 120000);
    await sleep(2500);
    await assertNoFullUiSyncFailures(hostPage, guestPage);
    const guestKeep = await clickLocalButton(guestPage, "guest-keep-consultation-test", /KEEP HAND/i);
    if (guestKeep) {
      await sleep(3000);
    }
    await assertNoFullUiSyncFailures(hostPage, guestPage);

    await driveDemonicConsultationCast({
      hostPage,
      guestPage,
      actorPage: hostPage,
      actorLabel: "host",
      maxSteps: 120,
    });
    await assertNoFullUiSyncFailures(hostPage, guestPage);
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS guest casting Demonic Consultation after playing Swamp keeps hidden openings synced", { timeout: 360000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const match = await startFullUiPeerMatch({
      baseUrl,
      hostContext,
      guestContext,
      hostDeckText: "60 Lightning Bolt",
      guestDeckText: "30 Swamp\n30 Demonic Consultation",
      hostName: "Alice P1",
      guestName: "Alice P2",
      hostLabel: "host-guest-consultation-ui",
      guestLabel: "guest-guest-consultation-ui",
    });
    hostPage = match.hostPage;
    guestPage = match.guestPage;

    await assertNoFullUiSyncFailures(hostPage, guestPage);
    await waitAndClickLocalButton(hostPage, "host-keep-guest-consultation-test", /KEEP HAND/i, 120000);
    await sleep(2500);
    await assertNoFullUiSyncFailures(hostPage, guestPage);
    const guestKeep = await clickLocalButton(guestPage, "guest-keep-guest-consultation-test", /KEEP HAND/i);
    if (guestKeep) {
      await sleep(3000);
    }
    await assertNoFullUiSyncFailures(hostPage, guestPage);

    await driveDemonicConsultationCast({
      hostPage,
      guestPage,
      actorPage: guestPage,
      actorLabel: "guest",
      maxSteps: 180,
    });
    await assertNoFullUiSyncFailures(hostPage, guestPage);
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS guest Demonic Consultation missing name exiles the library without desync", { timeout: 480000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const match = await startFullUiPeerMatch({
      baseUrl,
      hostContext,
      guestContext,
      hostDeckText: "60 Lightning Bolt",
      guestDeckText: "30 Swamp\n30 Demonic Consultation",
      hostName: "Alice P1",
      guestName: "Alice P2",
      hostLabel: "host-guest-consultation-missing-name",
      guestLabel: "guest-guest-consultation-missing-name",
    });
    hostPage = match.hostPage;
    guestPage = match.guestPage;

    await assertNoFullUiSyncFailures(hostPage, guestPage);
    await waitAndClickLocalButton(hostPage, "host-keep-guest-consultation-missing-name-test", /KEEP HAND/i, 120000);
    await sleep(2500);
    await assertNoFullUiSyncFailures(hostPage, guestPage);
    const guestKeep = await clickLocalButton(guestPage, "guest-keep-guest-consultation-missing-name-test", /KEEP HAND/i);
    if (guestKeep) {
      await sleep(3000);
    }
    await assertNoFullUiSyncFailures(hostPage, guestPage);

    await driveDemonicConsultationCast({
      hostPage,
      guestPage,
      actorPage: guestPage,
      actorLabel: "guest",
      maxSteps: 180,
    });
    await resolveDemonicConsultationWithMissingName({
      hostPage,
      guestPage,
      actorPage: guestPage,
      actorLabel: "guest",
      actorIndex: 1,
      missingName: "Black Lotus",
      maxSteps: 220,
    });
    await assertNoFullUiSyncFailures(hostPage, guestPage);
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS host Demonic Consultation missing name after mulligan keeps ziffle openings linked", { timeout: 480000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const match = await startFullUiPeerMatch({
      baseUrl,
      hostContext,
      guestContext,
      hostDeckText: "30 Swamp\n30 Demonic Consultation",
      guestDeckText: "60 Lightning Bolt",
      hostName: "Alice P1",
      guestName: "Alice P2",
      hostLabel: "host-consultation-mulligan-missing-name",
      guestLabel: "guest-consultation-mulligan-missing-name",
    });
    hostPage = match.hostPage;
    guestPage = match.guestPage;

    await assertNoFullUiSyncFailures(hostPage, guestPage);
    let hostMulligansTaken = 0;
    let hostHand = null;
    for (; hostMulligansTaken < 5; hostMulligansTaken += 1) {
      const mulliganNumber = hostMulligansTaken + 1;
      const beforeMulligan = Number((await fullUiSnapshot(hostPage))?.multiplayer?.lastAppliedSequence || 0);
      await waitAndClickLocalButton(
        hostPage,
        `host-consultation-missing-name-mulligan-${mulliganNumber}`,
        /Mulligan/i,
        120000,
      );
      await waitForFullUiSequenceAdvance(
        hostPage,
        guestPage,
        beforeMulligan,
        `expected host Demonic Consultation mulligan ${mulliganNumber} to sync`,
        180000,
      );
      if (mulliganNumber === 1) {
        await waitAndClickLocalButton(guestPage, "guest-keep-after-host-consultation-mulligan", /KEEP HAND/i, 120000);
        await sleep(2500);
        await assertNoFullUiSyncFailuresWithDebug(
          "guest keep after host Consultation mulligan should stay synced",
          hostPage,
          guestPage,
        );
      }

      hostHand = await waitForNamedVisibleHand(
        hostPage,
        `expected host redraw hand to be visible after Consultation mulligan ${mulliganNumber}`,
        180000,
      );
      if (hostHand.includes("Swamp") && hostHand.includes("Demonic Consultation")) {
        break;
      }
      await assertNoFullUiSyncFailuresWithDebug(
        `host Consultation mulligan ${mulliganNumber} produced a non-castable hand but stayed synced`,
        hostPage,
        guestPage,
      );
    }
    assert.ok(
      hostHand?.includes("Swamp") && hostHand.includes("Demonic Consultation"),
      `expected a host mulligan hand able to cast Demonic Consultation\nmulligans: ${hostMulligansTaken}\nhand: ${JSON.stringify(hostHand, null, 2)}`
    );
    hostMulligansTaken += 1;
    await waitAndClickLocalButton(hostPage, "host-keep-consultation-mulligan-hand", /KEEP HAND/i, 120000);
    await waitForVisibleBodyText(
      hostPage,
      /Choose \d+ card\(s\) to put on the bottom of your library/i,
      "expected host to bottom one card after the Consultation mulligan",
      120000,
    );
    const bottomPromptText = await visibleBodyText(hostPage);
    const bottomCount = Number(
      bottomPromptText.match(/Choose\s+(\d+)\s+card\(s\)\s+to put on the bottom of your library/i)?.[1]
      || hostMulligansTaken
    );
    const beforeBottom = Number((await fullUiSnapshot(hostPage))?.multiplayer?.lastAppliedSequence || 0);
    for (let bottomIndex = 0; bottomIndex < bottomCount; bottomIndex += 1) {
      const namesBeforeBottom = await visibleHandCardNames(hostPage);
      const swampCount = namesBeforeBottom.filter((name) => name === "Swamp").length;
      const consultationCount = namesBeforeBottom.filter((name) => name === "Demonic Consultation").length;
      const bottomPattern =
        consultationCount > 1
          ? /^Demonic Consultation$/i
          : swampCount > 1
            ? /^Swamp$/i
            : null;
      assert.ok(
        bottomPattern,
        `expected a redundant card to bottom while keeping Consultation castable\nbottom ${bottomIndex + 1}/${bottomCount}\nhand: ${JSON.stringify(namesBeforeBottom, null, 2)}`
      );
      const bottomChoice = await clickUnselectedEnabledButton(
        hostPage,
        `host-bottom-after-consultation-mulligan-${bottomIndex + 1}`,
        bottomPattern,
      );
      assert.ok(bottomChoice, `expected host to choose bottom card ${bottomIndex + 1}\n${await visibleBodyText(hostPage)}`);
      await sleep(250);
    }
    await waitAndClickLocalButton(hostPage, "host-submit-bottom-after-consultation-mulligan", /SUBMIT/i, 60000);
    await waitForFullUiSequenceAdvance(
      hostPage,
      guestPage,
      beforeBottom,
      "expected host bottom card after Consultation mulligan to sync",
      180000,
    );
    await assertNoFullUiSyncFailuresWithDebug(
      "host bottom card after Consultation mulligan should stay synced",
      hostPage,
      guestPage,
    );

    await driveDemonicConsultationCast({
      hostPage,
      guestPage,
      actorPage: hostPage,
      actorLabel: "host",
      maxSteps: 180,
    });
    await resolveDemonicConsultationWithMissingName({
      hostPage,
      guestPage,
      actorPage: hostPage,
      actorLabel: "host",
      actorIndex: 0,
      missingName: "Black Lotus",
      maxSteps: 240,
    });
    await assertNoFullUiSyncFailures(hostPage, guestPage);
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS Selvala after host mulligans reveals ziffle libraries without desync", { timeout: 600000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const screenshotDir = path.join(UI_ROOT, "test-results", "selvala-mulligan-e2e");
  fs.rmSync(screenshotDir, { recursive: true, force: true });
  fs.mkdirSync(screenshotDir, { recursive: true });
  let hostPage = null;
  let guestPage = null;
  let screenshotStep = 0;

  const capture = async (slug) => {
    screenshotStep += 1;
    return captureFullUiStep(screenshotDir, screenshotStep, slug, [
      ["p1-host", hostPage],
      ["p2-guest", guestPage],
    ]);
  };

  const combinedText = async () => `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;

  const assertClean = async (label) => {
    const text = await combinedText();
    try {
      assertNoSyncFailureText(text, label);
      assert.doesNotMatch(
        text,
        /Cheat detected|Invalid priority action ref|Action order mismatch|Missing public_open audit opening|Missing audit material/i,
        label,
      );
    } catch (err) {
      const debug = await Promise.all([
        fullUiPageDebug(hostPage, 0),
        fullUiPageDebug(guestPage, 1),
      ]);
      assert.fail(`${label}\n${err?.message || err}\n${JSON.stringify(debug, null, 2)}`);
    }
  };

  const clickUnselectedButton = async (page, label, textPattern) => {
    const button = page.locator("button:enabled:not(.is-selected)").filter({ hasText: textPattern }).first();
    if ((await button.count()) === 0) return null;
    const text = (await button.innerText({ timeout: 1000 })).replace(/\s+/g, " ").trim();
    await activateButtonNode(button);
    return { label, text };
  };

  const unselectedChoiceTexts = async (page) =>
    page.locator("button:enabled:not(.is-selected)").evaluateAll((buttons) =>
      buttons.map((button) => (button.innerText || button.textContent || "").replace(/\s+/g, " ").trim())
    );

  const visibleHandCardNames = async (page) =>
    page.locator(".hand-card[data-card-name]").evaluateAll((cards) =>
      cards
        .map((card) => String(card.getAttribute("data-card-name") || "").trim())
        .filter(Boolean)
    );

  const battlefieldCardCount = async (page, cardName) =>
    page.locator(".battlefield-row-card[data-card-name]").evaluateAll((cards, name) =>
      cards.filter((card) => String(card.getAttribute("data-card-name") || "") === name).length,
      cardName,
    );

  const stackCardCount = async (page, cardName) =>
    page.locator(".stack-card[data-card-name]").evaluateAll((cards, name) =>
      cards.filter((card) => String(card.getAttribute("data-card-name") || "") === name).length,
      cardName,
    );

  const castableSelvalaSetupHand = async () => {
    const names = await visibleHandCardNames(hostPage);
    const lotusCount = names.filter((name) => /^Black Lotus$/i.test(name)).length;
    const selvalaCount = names.filter((name) => /^Selvala, Explorer Returned$/i.test(name)).length;
    const hiddenCount = names.filter((name) => /^Hidden Card$/i.test(name)).length;
    return {
      names,
      lotusCount,
      selvalaCount,
      hiddenCount,
      castable: lotusCount >= 2 && (selvalaCount + hiddenCount) >= 1,
    };
  };

  const bottomOneCardKeepingCastableHand = async (index) => {
    const choices = await unselectedChoiceTexts(hostPage);
    const lotusCount = choices.filter((text) => /^Black Lotus\b/i.test(text)).length;
    const selvalaCount = choices.filter((text) => /^Selvala, Explorer Returned\b/i.test(text)).length;
    const hiddenCount = choices.filter((text) => /^Hidden Card\b/i.test(text)).length;
    let pattern = null;
    if (selvalaCount > 1) pattern = /^Selvala, Explorer Returned\b/i;
    else if (selvalaCount + hiddenCount > 1 && hiddenCount > 0) pattern = /^Hidden Card\b/i;
    else if (lotusCount > 2) pattern = /^Black Lotus\b/i;
    assert.ok(
      pattern,
      `mulligan redraw must leave at least two Black Lotuses and one Selvala-or-hidden deck card after bottoming\nchoices: ${JSON.stringify(choices, null, 2)}\nbody:\n${await visibleBodyText(hostPage)}`
    );
    const clicked = await clickUnselectedButton(hostPage, `host-bottom-card-${index}`, pattern);
    assert.ok(clicked, `expected to select bottom card ${index}\nchoices: ${JSON.stringify(choices, null, 2)}`);
  };

  const advanceUntil = async (predicate, label, maxSteps = 160) => {
    const advancePattern = /KEEP HAND|PREGAME|BEGIN GAME|CONTINUE|UPKEEP|DRAW|MAIN|COMBAT|M2|END|CLEAN|PASS PRIORITY|RESOLVE|NO ATTACKERS|DECLARE ATTACKERS|DECLARE BLOCKERS|CONFIRM ATTACKERS|CONFIRM BLOCKERS|DONE|SUBMIT/i;
    let lastHostText = "";
    let lastGuestText = "";
    for (let step = 0; step < maxSteps; step += 1) {
      lastHostText = await visibleBodyText(hostPage);
      lastGuestText = await visibleBodyText(guestPage);
      await assertClean(label);
      const hostButtons = await buttonDebugText(hostPage);
      const guestButtons = await buttonDebugText(guestPage);
      if (await predicate({ hostText: lastHostText, guestText: lastGuestText, hostButtons, guestButtons })) {
        return { hostText: lastHostText, guestText: lastGuestText, hostButtons, guestButtons };
      }

      const bottomPromptMatch = lastHostText.match(/Choose (\d+) card\(s\) to put on the bottom of your library/i);
      if (bottomPromptMatch) {
        const required = Number(bottomPromptMatch[1]);
        const submitButton = hostButtons.find((button) => /SUBMIT/i.test(button.text || ""));
        const submitProgress = String(submitButton?.text || "").match(/\((\d+)\/(\d+)\)/);
        const selected = submitProgress ? Number(submitProgress[1]) : 0;
        const target = submitProgress ? Number(submitProgress[2]) : required;
        for (let index = selected + 1; index <= target; index += 1) {
          await bottomOneCardKeepingCastableHand(index);
        }
        await capture("p1-selected-bottom-cards-late");
        const submittedBottom = await clickLocalButton(hostPage, `${label}-host-submit-late-bottom-${step}`, /SUBMIT/i);
        assert.ok(
          submittedBottom,
          `expected late bottom-card decision to be submittable\nbuttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}\nbody:\n${await visibleBodyText(hostPage)}`
        );
        await sleep(3000);
        await capture("p1-bottomed-cards-late");
        continue;
      }

      const hostDiscard = /Discard \d+ card/i.test(lastHostText)
        ? await clickEnabledButton(hostPage, `${label}-host-discard-choice-${step}`, /^(Black Lotus|Selvala, Explorer Returned)$/i)
        : null;
      if (hostDiscard) {
        await sleep(500);
        const submit = await clickLocalButton(hostPage, `${label}-host-submit-discard-${step}`, /SUBMIT/i);
        if (submit) {
          await sleep(2200);
          continue;
        }
      }

      const guestDiscard = /Discard \d+ card/i.test(lastGuestText)
        ? await clickEnabledButton(guestPage, `${label}-guest-discard-choice-${step}`, /^(Black Lotus|Selvala, Explorer Returned)$/i)
        : null;
      if (guestDiscard) {
        await sleep(500);
        const submit = await clickLocalButton(guestPage, `${label}-guest-submit-discard-${step}`, /SUBMIT/i);
        if (submit) {
          await sleep(2200);
          continue;
        }
      }

      const hostAdvanced = await clickLocalButton(hostPage, `${label}-host-advance-${step}`, advancePattern);
      if (hostAdvanced) {
        await sleep(2200);
        continue;
      }
      const guestAdvanced = await clickLocalButton(guestPage, `${label}-guest-advance-${step}`, advancePattern);
      if (guestAdvanced) {
        await sleep(2200);
        continue;
      }
      await sleep(1000);
    }
    const debug = await Promise.all([
      fullUiPageDebug(hostPage, 0),
      fullUiPageDebug(guestPage, 1),
    ]);
    assert.fail(`${label}\n${JSON.stringify(debug, null, 2)}`);
  };

  const waitForStackCardToResolve = async (cardName, expectedBattlefieldCount, label, maxSteps = 90) => {
    for (let step = 0; step < maxSteps; step += 1) {
      await assertClean(label);
      const hostText = await visibleBodyText(hostPage);
      const hostBattlefieldCount = await battlefieldCardCount(hostPage, cardName);
      const hostStackCount = await stackCardCount(hostPage, cardName);
      const guestStackCount = await stackCardCount(guestPage, cardName);
      const expectedBattlefieldSummary = new RegExp(`BF\\s*${expectedBattlefieldCount}\\s*HAND`, "i").test(hostText);
      if (
        (hostBattlefieldCount >= expectedBattlefieldCount || expectedBattlefieldSummary)
        && hostStackCount === 0
        && guestStackCount === 0
      ) {
        return;
      }
      const hostPass = await clickLocalButton(hostPage, `${label}-host-pass-${step}`, /PASS PRIORITY|RESOLVE/i);
      if (hostPass) {
        await sleep(2200);
        continue;
      }
      const guestPass = await clickLocalButton(guestPage, `${label}-guest-pass-${step}`, /PASS PRIORITY|RESOLVE/i);
      if (guestPass) {
        await sleep(2200);
        continue;
      }
      await sleep(1000);
    }
    const debug = await Promise.all([
      fullUiPageDebug(hostPage, 0),
      fullUiPageDebug(guestPage, 1),
    ]);
    assert.fail(`${label}\n${JSON.stringify(debug, null, 2)}`);
  };

  const castBlackLotus = async (ordinal) => {
    await advanceUntil(
      async () => hasLocalButton(hostPage, /CAST BLACK LOTUS/i),
      `expected Player 1 to have Black Lotus ${ordinal} castable`,
      180,
    );
    const beforeCastSequence = Number((await fullUiSnapshot(hostPage))?.multiplayer?.lastAppliedSequence || 0);
    await waitAndClickLocalButton(hostPage, `host-cast-black-lotus-${ordinal}`, /CAST BLACK LOTUS/i, 120000);
    await waitForFullUiSequenceAdvance(
      hostPage,
      guestPage,
      beforeCastSequence,
      `Black Lotus ${ordinal} cast should sync before resolution`,
    );
    await waitForStackCardToResolve("Black Lotus", ordinal, `resolve Black Lotus ${ordinal}`);
    await waitForFullUiSync(
      hostPage,
      guestPage,
      `Black Lotus ${ordinal} resolution should leave peers synced`,
      120000,
    );
    await assertClean(`casting Black Lotus ${ordinal} should not sync-fail`);
  };

  const paySelvalaCost = async () => {
    let choseGreenLotus = false;
    const paymentPatterns = [
      /BLACK LOTUS/i,
      /Use\s+from mana pool/i,
      /GREEN|\{G\}/i,
      /WHITE|\{W\}/i,
      /GENERIC|\{1\}|MANA|PAY|SUBMIT/i,
      /^CAST$/i,
    ];
    for (let step = 0; step < 40; step += 1) {
      const text = await visibleBodyText(hostPage);
      await assertClean("Selvala payment should stay synced");
      const paymentPending = /Pay \{|remaining|Use\s+from mana pool|CHOOSE OPTION|CHOOSE COLOR|Choose a color/i.test(text);
      if (!paymentPending && await stackCardCount(hostPage, "Selvala, Explorer Returned") > 0) return;
      const submitted = await clickEnabledButton(
        hostPage,
        `host-pay-selvala-submit-${step}`,
        /^SUBMIT(?:\s*\(\d+\/\d+\))?$|^PAY$/i,
      );
      if (submitted) {
        await sleep(800);
        continue;
      }
      if (/CHOOSE COLOR|Choose a color/i.test(text)) {
        const colorPattern = choseGreenLotus ? /WHITE|\{W\}/i : /GREEN|\{G\}/i;
        const clickedColor =
          await clickUnselectedEnabledButton(hostPage, `host-pay-selvala-color-${step}`, colorPattern)
          || await clickEnabledButton(hostPage, `host-pay-selvala-color-${step}`, colorPattern);
        if (clickedColor) {
          if (!choseGreenLotus) choseGreenLotus = true;
          await sleep(800);
          continue;
        }
      }
      for (const pattern of paymentPatterns) {
        const clicked =
          await clickUnselectedEnabledButton(hostPage, `host-pay-selvala-${step}`, pattern)
          || await clickEnabledButton(hostPage, `host-pay-selvala-${step}`, pattern);
        if (clicked) {
          await sleep(800);
          break;
        }
      }
      const nextText = await visibleBodyText(hostPage);
      const nextPaymentPending = /Pay \{|remaining|Use\s+from mana pool|CHOOSE OPTION|CHOOSE COLOR|Choose a color/i.test(nextText);
      if (!nextPaymentPending && await stackCardCount(hostPage, "Selvala, Explorer Returned") > 0) return;
      await sleep(500);
    }
    assert.fail(
      `expected Selvala payment to put Selvala on the stack\nbuttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}\nbody:\n${await visibleBodyText(hostPage)}`
    );
  };

  const priorityCommandForSnapshotAction = (action) => {
    if (action?.action_ref) {
      return { type: "priority_action", action_ref: action.action_ref };
    }
    const actionIndex = Number(action?.index);
    return Number.isSafeInteger(actionIndex)
      ? { type: "priority_action", action_index: actionIndex }
      : null;
  };

  const actionKind = (action) => String(action?.action_ref?.kind || "").trim();

  const activateSelvalaWithFastAdvance = async () => {
    let lastProgress = "Selvala resolved";
    for (let step = 0; step < 160; step += 1) {
      await assertClean("Selvala fast turn advance should stay synced");
      const hostSnapshot = await fullUiSnapshot(hostPage);
      const state = hostSnapshot?.state || {};
      const decision = state.decision || {};
      const decisionPlayer = decision.player == null ? null : Number(decision.player);
      const actorPage = decisionPlayer === 1 ? guestPage : hostPage;
      const actions = Array.isArray(state.decisionActions) ? state.decisionActions : [];
      const selvalaId = (state.players?.[0]?.battlefield || [])
        .find((card) => /^Selvala, Explorer Returned$/i.test(String(card.name || "")))?.id;

      if (decision.kind === "priority" && decisionPlayer === 0 && selvalaId != null) {
        const activateAction = actions.find((action) =>
          (actionKind(action) === "activate_ability" || actionKind(action) === "activate_mana_ability")
          && Number(action?.action_ref?.source) === Number(selvalaId)
        );
        const activateCommand = priorityCommandForSnapshotAction(activateAction);
        if (activateCommand) {
          const beforeSequence = Number(hostSnapshot?.multiplayer?.lastAppliedSequence || 0);
          await submitFullUiMultiplayerCommand(
            hostPage,
            activateCommand,
            "Activate Selvala parley",
          );
          await waitForFullUiSequenceAdvance(
            hostPage,
            guestPage,
            beforeSequence,
            "Selvala activation should sync",
            120000,
          );
          await sleep(2500);
          return;
        }
      }

      if (decision.kind === "priority") {
        const passAction = actions.find((action) => actionKind(action) === "pass_priority") || actions[0];
        const passCommand = priorityCommandForSnapshotAction(passAction);
        if (passCommand && actorPage) {
          lastProgress = `passing priority for player ${decisionPlayer}`;
          await submitFullUiMultiplayerCommand(
            actorPage,
            passCommand,
            "Pass priority",
            { timeoutMs: 60000 },
          );
          await sleep(900);
          continue;
        }
      }

      if (
        (decision.kind === "attackers" || decision.kind === "blockers")
        && actorPage
      ) {
        lastProgress = `declaring no ${decision.kind} for player ${decisionPlayer}`;
        await submitFullUiMultiplayerCommand(
          actorPage,
          {
            type: decision.kind === "attackers" ? "declare_attackers" : "declare_blockers",
            declarations: [],
          },
          decision.kind === "attackers" ? "Declared 0 attacker(s)" : "Declared 0 blocker(s)",
          { timeoutMs: 60000 },
        );
        await sleep(900);
        continue;
      }

      if (
        decision.kind === "select_objects"
        && /discard|bottom/i.test(String(decision.description || ""))
        && actorPage
      ) {
        const candidate = (state.decisionCandidates || []).find((entry) => entry.legal !== false);
        if (candidate?.id != null) {
          lastProgress = `submitting ${decision.description}`;
          await submitFullUiMultiplayerCommand(
            actorPage,
            { type: "select_objects", object_ids: [Number(candidate.id)] },
            "Selected 1 object(s)",
            { timeoutMs: 60000 },
          );
          await sleep(900);
          continue;
        }
      }

      const hostAdvanced = await clickLocalButton(hostPage, `selvala-fast-advance-host-${step}`, /PASS PRIORITY|RESOLVE|UPKEEP|DRAW|MAIN|COMBAT|M2|END|CLEAN|NO ATTACKERS|DECLARE ATTACKERS|DECLARE BLOCKERS|DONE|SUBMIT/i);
      if (hostAdvanced) {
        lastProgress = `clicked host ${hostAdvanced.text}`;
        await sleep(1200);
        continue;
      }
      const guestAdvanced = await clickLocalButton(guestPage, `selvala-fast-advance-guest-${step}`, /PASS PRIORITY|RESOLVE|UPKEEP|DRAW|MAIN|COMBAT|M2|END|CLEAN|NO ATTACKERS|DECLARE ATTACKERS|DECLARE BLOCKERS|DONE|SUBMIT/i);
      if (guestAdvanced) {
        lastProgress = `clicked guest ${guestAdvanced.text}`;
        await sleep(1200);
        continue;
      }

      await sleep(800);
    }

    const debug = await Promise.all([
      fullUiPageDebug(hostPage, 0),
      fullUiPageDebug(guestPage, 1),
    ]);
    assert.fail(`expected Selvala activation to become legal after a turn cycle (${lastProgress})\n${JSON.stringify(debug, null, 2)}`);
  };

  const activateSelvalaAndAcknowledgeReveal = async (slug, expectedPhasePattern = null, options = {}) => {
    if (!options.alreadyActivated) {
      await waitAndClickLocalButton(
        hostPage,
        `host-activate-selvala-${slug}`,
        /Each player reveals the top card of their library/i,
        120000,
      );
    }
    await sleep(4000);
    try {
      await Promise.all([
        waitForVisibleBodyText(
          hostPage,
          /REVEALED[\s\S]*(Black Lotus|Selvala, Explorer Returned)/i,
          `expected host to see Selvala reveal during ${slug}`,
          120000,
        ),
        waitForVisibleBodyText(
          guestPage,
          /REVEALED[\s\S]*(Black Lotus|Selvala, Explorer Returned)/i,
          `expected guest to see Selvala reveal during ${slug}`,
          120000,
        ),
      ]);
    } catch (err) {
      const debug = await Promise.all([
        fullUiPageDebug(hostPage, 0),
        fullUiPageDebug(guestPage, 1),
      ]);
      assert.fail(`${err?.message || err}\n${JSON.stringify(debug, null, 2)}`);
    }
    await capture(`${slug}-reveal-visible`);

    const guestRevealText = await visibleBodyText(guestPage);
    assert.doesNotMatch(
      guestRevealText,
      /REVEALED[\s\S]{0,260}Hidden Card/i,
      `guest reveal strip should show opened card names during ${slug}`,
    );
    await assertClean(`Selvala ${slug} reveal should not desync`);

    await clickEnabledButton(hostPage, `host-done-${slug}`, /^DONE$/i);
    await sleep(1200);
    await clickEnabledButton(guestPage, `guest-done-${slug}`, /^DONE$/i);
    await sleep(3000);
    await capture(`${slug}-after-done`);
    const afterDoneHost = await visibleBodyText(hostPage);
    await assertClean(`Selvala ${slug} Done should not desync or auto-pass incorrectly`);
    if (expectedPhasePattern) {
      assert.match(
        afterDoneHost,
        expectedPhasePattern,
        `expected ${slug} Done to leave priority in the expected phase\n${afterDoneHost}`,
      );
    }
  };

  try {
    const deck = deckUrlParam("30 Black Lotus\n30 Selvala, Explorer Returned");
    hostPage = await openFullUiPage(
      hostContext,
      `${baseUrl}/?name=Alice P1&deck=${deck}`,
      "host-selvala-mulligan-ui",
    );
    await hostPage.getByText("CREATE LOBBY").first().waitFor({ timeout: 30000 });
    await capture("host-load");
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).first().click();
    await hostPage.getByText("Host or join").waitFor({ timeout: 10000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).last().click();
    await hostPage.getByText("Share this code").waitFor({ timeout: 40000 });
    await capture("host-created-lobby");

    const lobbyCode = (await visibleBodyText(hostPage)).match(
      /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i
    )?.[0];
    assert.ok(lobbyCode, `expected the full UI to create a lobby code\n${await visibleBodyText(hostPage)}`);

    guestPage = await openFullUiPage(
      guestContext,
      `${baseUrl}/?lobby=${encodeURIComponent(lobbyCode)}&name=Alice P2&deck=${deck}`,
      "guest-selvala-mulligan-ui",
    );
    await hostPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });
    await guestPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });
    await capture("guest-joined-lobby");

    await hostPage.getByRole("button").filter({ hasText: /START GAME/i }).click();
    await hostPage.getByRole("button").filter({ hasText: /START GAME/i }).waitFor({
      state: "detached",
      timeout: 60000,
    }).catch(() => {});
    await sleep(8000);
    await Promise.all([
      hostPage.keyboard.press("Escape").catch(() => {}),
      guestPage.keyboard.press("Escape").catch(() => {}),
    ]);
    await sleep(500);
    await capture("opening-hand");
    await assertClean("match start should not sync-fail before Selvala mulligans");

    await waitAndClickLocalButton(hostPage, "host-first-mulligan", /Mulligan/i, 120000);
    await sleep(3000);
    await capture("p1-first-mulligan");
    await assertClean("host first mulligan should not sync-fail");

    await waitAndClickLocalButton(guestPage, "guest-keep-after-host-first-mulligan", /KEEP HAND/i, 120000);
    await sleep(3000);
    await capture("p2-kept-opening-hand");
    await assertClean("guest keep after host mulligan should not sync-fail");

    await waitAndClickLocalButton(hostPage, "host-second-mulligan", /Mulligan/i, 120000);
    await sleep(5000);
    await capture("p1-second-mulligan");
    await assertClean("host second mulligan should not sync-fail");

    let hostMulliganCount = 2;
    let setupHand = await castableSelvalaSetupHand();
    while (!setupHand.castable && hostMulliganCount < 10) {
      await waitAndClickLocalButton(
        hostPage,
        `host-extra-mulligan-${hostMulliganCount + 1}-for-selvala-setup`,
        /Mulligan/i,
        120000,
      );
      hostMulliganCount += 1;
      await sleep(5000);
      await capture(`p1-mulligan-${hostMulliganCount}-for-selvala-setup`);
      await assertClean(`host mulligan ${hostMulliganCount} should not sync-fail`);
      setupHand = await castableSelvalaSetupHand();
    }
    assert.ok(
      setupHand.castable,
      `expected a mulligan hand with at least two Black Lotuses and one Selvala-or-hidden deck card before keeping\nhand: ${JSON.stringify(setupHand.names, null, 2)}`
    );

    await waitAndClickLocalButton(hostPage, "host-keep-after-selvala-setup-hand", /KEEP HAND/i, 120000);
    await sleep(2500);
    await capture(`p1-kept-after-${hostMulliganCount}-mulligans`);
    await assertClean("host keep after mulligans should not sync-fail");

    await sleep(7000);
    const afterKeepFlowText = await waitForVisibleBodyText(
      hostPage,
      /Choose \d+ card\(s\) to put on the bottom of your library|BEGINNING|UNTAP|DRAW|MAIN|CAST BLACK LOTUS/i,
      "expected Player 1 to either bottom cards or advance into the game after mulligans",
      120000,
    );
    const bottomPromptMatch = afterKeepFlowText.match(/Choose (\d+) card\(s\) to put on the bottom of your library/i);
    if (bottomPromptMatch) {
      const bottomCount = Number(bottomPromptMatch[1]);
      assert.ok(Number.isFinite(bottomCount) && bottomCount >= 0, `invalid bottom prompt: ${bottomPromptMatch[0]}`);
      for (let index = 1; index <= bottomCount; index += 1) {
        await bottomOneCardKeepingCastableHand(index);
      }
      await capture("p1-selected-bottom-cards");
      await waitAndClickLocalButton(hostPage, "host-submit-bottom-cards", /SUBMIT/i, 60000);
      await sleep(4000);
      await capture("p1-bottomed-cards");
      await assertClean("bottoming cards after mulligans should not sync-fail");
    } else {
      await capture("p1-advanced-without-bottom-prompt");
      await assertClean("advancing after mulligans without a bottom prompt should not sync-fail");
    }

    await advanceUntil(
      async () => hasLocalButton(hostPage, /CAST BLACK LOTUS/i),
      "expected Player 1 to reach a main phase with Black Lotus castable",
      160,
    );
    await capture("p1-main-ready-to-cast-lotus");

    await castBlackLotus(1);
    await capture("first-black-lotus-resolved");
    await castBlackLotus(2);
    await capture("second-black-lotus-resolved");

    const beforeSelvalaCastSequence = Number((await fullUiSnapshot(hostPage))?.multiplayer?.lastAppliedSequence || 0);
    await waitAndClickLocalButton(hostPage, "host-cast-selvala", /CAST SELVALA, EXPLORER RETURNED/i, 120000);
    await waitForFullUiSequenceAdvance(
      hostPage,
      guestPage,
      beforeSelvalaCastSequence,
      "Selvala cast should sync before payment",
      120000,
    );
    await paySelvalaCost();
    await capture("selvala-on-stack");
    await waitForStackCardToResolve("Selvala, Explorer Returned", 1, "resolve Selvala", 120);
    await advanceUntil(
      async ({ hostText }) => /BF\s*1/i.test(hostText) && /Selvala, Explorer Returned/i.test(hostText),
      "expected Selvala to resolve to Player 1 battlefield",
      80,
    );
    await capture("selvala-on-battlefield");

    await activateSelvalaWithFastAdvance();
    await capture("selvala-activation-submitted");
    await activateSelvalaAndAcknowledgeReveal(
      "post-summoning-sickness-selvala",
      null,
      { alreadyActivated: true },
    );
    await capture("final-after-selvala-reveal");

    const finalCombinedText = await combinedText();
    assertNoSyncFailureText(finalCombinedText, "Selvala full UI ziffle reveal flow should stay synced");
    assert.doesNotMatch(finalCombinedText, /Hidden Card commitment does not match reveal|Ziffle card opening proof/i);
    assertNoPageErrors(hostPage, guestPage);
    console.log(`SELVALA_MULLIGAN_E2E_SCREENSHOTS=${screenshotDir}`);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS Mystical Tutor resolves into a searchable hidden library choice", { timeout: 300000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const hostDeck = deckUrlParam("30 Island\n30 Mystical Tutor");
    const guestDeck = deckUrlParam("60 Mountain");
    hostPage = await openFullUiPage(hostContext, `${baseUrl}/?name=Chiplis&deck=${hostDeck}`, "host-ui");
    await hostPage.getByText("CREATE LOBBY").first().waitFor({ timeout: 30000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).first().click();
    await hostPage.getByText("Host or join").waitFor({ timeout: 10000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).last().click();
    await hostPage.getByText("Share this code").waitFor({ timeout: 40000 });

    const lobbyCode = (await visibleBodyText(hostPage)).match(
      /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i
    )?.[0];
    assert.ok(lobbyCode, "expected the full UI to create a lobby code");

    guestPage = await openFullUiPage(
      guestContext,
      `${baseUrl}/?lobby=${encodeURIComponent(lobbyCode)}&name=Alice&deck=${guestDeck}`,
      "guest-ui",
    );
    await hostPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });
    await guestPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });

    await hostPage.getByRole("button").filter({ hasText: /START GAME/i }).click();
    await hostPage.getByRole("button").filter({ hasText: /START GAME/i }).waitFor({
      state: "detached",
      timeout: 60000,
    }).catch(() => {});
    await sleep(8000);
    await Promise.all([
      hostPage.keyboard.press("Escape").catch(() => {}),
      guestPage.keyboard.press("Escape").catch(() => {}),
    ]);
    await sleep(500);

    let playedIsland = false;
    let castTutor = false;
    let paidTutor = false;
    let guestResolved = false;
    const stackCardCount = async (page, cardName) =>
      page.locator(".stack-card[data-card-name]").evaluateAll((cards, name) =>
        cards.filter((card) => String(card.getAttribute("data-card-name") || "") === name).length,
        cardName,
      );
    const finishTutorSearchIfVisible = async () => {
      const searchText = await visibleBodyText(hostPage);
      if (!/(?:CHOOSE OBJECTS[\s\S]*)?Search library[\s\S]*Mystical Tutor/i.test(searchText)) {
        return false;
      }
      guestResolved = true;
      assertNoSyncFailureText(searchText, "search decision should not sync-fail");
      await waitForFullUiPair(
        hostPage,
        guestPage,
        (host, guest) =>
          Number(host?.multiplayer?.lastAppliedSequence || 0)
            === Number(guest?.multiplayer?.lastAppliedSequence || 0)
          && host?.state?.decision?.kind === "select_objects"
          && guest?.state?.decision?.kind === "select_objects",
        "expected Mystical Tutor search decision to be synced before choosing",
        60000,
      );

      const choice = await hostPage.evaluate(() => {
        const snap = window.__ironsmithE2E?.snapshot?.();
        const candidate = (snap?.state?.decisionCandidates || []).find((entry) =>
          entry.legal !== false && /^Mystical Tutor$/i.test(String(entry.name || ""))
        );
        return candidate?.id == null ? null : { id: Number(candidate.id) };
      });
      assert.ok(
        choice?.id != null,
        `expected a Mystical Tutor choice in the searched library\nbuttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}\nbody:\n${await visibleBodyText(hostPage)}`
      );
      return true;
    };

    for (let step = 0; step < 120; step += 1) {
      const combinedText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
      assertNoSyncFailureText(combinedText);

      if (!playedIsland && await clickLocalButton(hostPage, "host-play-island", /PLAY ISLAND/i)) {
        playedIsland = true;
        await sleep(3000);
        continue;
      }

      if (playedIsland && !castTutor) {
        const hostText = await visibleBodyText(hostPage);
        if (
          /Pay [\s\S]*Mystical Tutor|CHOOSE OPTION/i.test(hostText)
          || await stackCardCount(hostPage, "Mystical Tutor") > 0
        ) {
          castTutor = true;
          continue;
        }
        if (await clickLocalButton(hostPage, "host-cast-tutor", /MYSTICAL TUTOR/i)) {
          await sleep(3000);
          continue;
        }
      }

      if (castTutor && !paidTutor) {
        const tutorPaymentPending = /Pay [\s\S]*Mystical Tutor|CHOOSE OPTION|remaining|Use\s+from mana pool/i.test(await visibleBodyText(hostPage));
        if (!tutorPaymentPending && await stackCardCount(hostPage, "Mystical Tutor") > 0) {
          paidTutor = true;
          await sleep(1500);
          continue;
        }
        const paid = await clickEnabledButton(hostPage, "host-pay-tutor", /Use\s+from mana pool|PAY|MANA|BLUE|ISLAND|\{U\}|U$|^CAST$/i)
          || await clickLocalButton(hostPage, "host-tap-island", /TAP ISLAND|ADD/i);
        if (paid) {
          await sleep(6000);
          continue;
        }
      }

      if (paidTutor && !guestResolved) {
        if (await finishTutorSearchIfVisible()) {
          break;
        }
        const hostPass = await clickLocalButton(hostPage, "host-pass-tutor", /PASS PRIORITY|RESOLVE/i);
        if (hostPass) {
          await sleep(2500);
          continue;
        }
        const guestPass = await clickLocalButton(guestPage, "guest-resolve-tutor", /RESOLVE|PASS/i);
        if (guestPass) {
          guestResolved = true;
          const searchText = await waitForVisibleBodyText(
            hostPage,
            /(?:CHOOSE OBJECTS[\s\S]*)?Search library[\s\S]*Mystical Tutor/i,
            "expected Mystical Tutor to create a searchable library decision",
            90000,
          );
          assert.ok(searchText);
          await finishTutorSearchIfVisible();
          break;
        }
      }

      if (!playedIsland) {
        const hostAdvanced = await clickLocalButton(hostPage, "host-setup", /KEEP HAND|PREGAME|UPKEEP|DRAW|PASS PRIORITY|RESOLVE/i);
        if (hostAdvanced) {
          await sleep(4500);
          continue;
        }
        const guestAdvanced = await clickLocalButton(guestPage, "guest-setup", /KEEP HAND|PREGAME|UPKEEP|DRAW|PASS PRIORITY|RESOLVE/i);
        if (guestAdvanced) {
          await sleep(4500);
          continue;
        }
      }

      await sleep(1000);
    }

    assert.equal(playedIsland, true, "expected host to play Island");
    assert.equal(castTutor, true, "expected host to cast Mystical Tutor");
    assert.equal(paidTutor, true, "expected host to pay for Mystical Tutor");
    assert.equal(
      guestResolved,
      true,
      `expected guest to resolve Mystical Tutor\nhost buttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}\nguest buttons: ${JSON.stringify(await buttonDebugText(guestPage), null, 2)}\nhost body:\n${await visibleBodyText(hostPage)}\nguest body:\n${await visibleBodyText(guestPage)}`
    );
    assertNoSyncFailureText(
      `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`,
      "Mystical Tutor should reach the searchable library choice without desync"
    );
    return;

    // eslint-disable-next-line no-unreachable
    const resolvedText = await waitForVisibleBodyText(
      hostPage,
      /GY\s*1[\s\S]*Mystical Tutor[\s\S]*Instant[\s\S]*Graveyard/i,
      "expected Mystical Tutor to finish resolving into its owner's graveyard",
      90000,
    );
    const guestResolvedText = await waitForVisibleBodyText(
      guestPage,
      /CHIPLIS[\s\S]*GY\s*1/i,
      "expected guest peer to apply the resolved Mystical Tutor",
      90000,
    );
    const finalCombinedText = `${resolvedText}\n${guestResolvedText}`;
    assertNoSyncFailureText(finalCombinedText, "Mystical Tutor full flow should stay synced");
    assert.doesNotMatch(finalCombinedText, /1 stack entry|STACK\s*1/i);

    let drewToppedCard = false;
    let lastAdvanceText = "";
    for (let step = 0; step < 180; step += 1) {
      const combinedText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
      lastAdvanceText = combinedText;
      assertNoSyncFailureText(combinedText, "drawing the Mystical Tutor target should stay synced");
      if (/CHIPLIS[\s\S]*HAND\s*6[\s\S]*GY\s*1[\s\S]*DECK\s*52/i.test(combinedText)) {
        drewToppedCard = true;
        break;
      }
      if (/DISCARD[\s\S]*Discard 1 card/i.test(combinedText)) {
        const hostDiscardChoice = await clickEnabledButton(hostPage, "host-discard-choice", /^(Island|Mountain|Mystical Tutor)$/i);
        if (hostDiscardChoice) {
          const hostDiscardSubmit = await clickLocalButton(hostPage, "host-submit-discard", /SUBMIT/i);
          if (hostDiscardSubmit) {
            await sleep(2200);
            continue;
          }
        }
        const guestDiscardChoice = await clickEnabledButton(guestPage, "guest-discard-choice", /^(Island|Mountain|Mystical Tutor)$/i);
        if (guestDiscardChoice) {
          const guestDiscardSubmit = await clickLocalButton(guestPage, "guest-submit-discard", /SUBMIT/i);
          if (guestDiscardSubmit) {
            await sleep(2200);
            continue;
          }
        }
      }

      const hostClicked = await clickLocalDecisionButton(hostPage, "host-advance-after-tutor");
      if (hostClicked) {
        await sleep(2200);
        continue;
      }
      const guestClicked = await clickLocalDecisionButton(guestPage, "guest-advance-after-tutor");
      if (guestClicked) {
        await sleep(2200);
        continue;
      }
      await sleep(1000);
    }
    assert.equal(
      drewToppedCard,
      true,
      `expected Player 0 to draw the card Mystical Tutor put on top\n${lastAdvanceText}`
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS Gitaxian Probe shows the targeted player's hand to the caster", { timeout: 240000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const hostDeck = deckUrlParam("60 Gitaxian Probe");
    const guestDeck = deckUrlParam("60 Lightning Bolt");
    hostPage = await openFullUiPage(hostContext, `${baseUrl}/?name=Chiplis&deck=${hostDeck}`, "host-ui");
    await hostPage.getByText("CREATE LOBBY").first().waitFor({ timeout: 30000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).first().click();
    await hostPage.getByText("Host or join").waitFor({ timeout: 10000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).last().click();
    await hostPage.getByText("Share this code").waitFor({ timeout: 40000 });

    const lobbyCode = (await visibleBodyText(hostPage)).match(
      /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i
    )?.[0];
    assert.ok(lobbyCode, "expected the full UI to create a lobby code");

    guestPage = await openFullUiPage(
      guestContext,
      `${baseUrl}/?lobby=${encodeURIComponent(lobbyCode)}&name=Alice&deck=${guestDeck}`,
      "guest-ui",
    );
    await hostPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });
    await guestPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });

    await hostPage.getByRole("button").filter({ hasText: /START GAME/i }).click();
    await hostPage.getByRole("button").filter({ hasText: /START GAME/i }).waitFor({
      state: "detached",
      timeout: 60000,
    }).catch(() => {});
    await sleep(8000);
    await Promise.all([
      hostPage.keyboard.press("Escape").catch(() => {}),
      guestPage.keyboard.press("Escape").catch(() => {}),
    ]);
    await sleep(500);

    let castProbe = false;
    let targetedGuest = false;
    let paidProbe = false;
    let guestResolved = false;
    let lastCombinedText = "";

    for (let step = 0; step < 70 && !guestResolved; step += 1) {
      const hostText = await visibleBodyText(hostPage);
      const guestText = await visibleBodyText(guestPage);
      const combinedText = `${hostText}\n${guestText}`;
      lastCombinedText = combinedText;
      assertNoSyncFailureText(combinedText);

      if (
        castProbe
        && targetedGuest
        && !paidProbe
        && /PRIORITY\s+ALICE/i.test(hostText)
        && /STACK[\s\S]*Gitaxian Probe/i.test(hostText)
      ) {
        paidProbe = true;
      }

      if (!castProbe && await clickLocalButton(hostPage, "host-cast-probe", /GITAXIAN PROBE/i)) {
        castProbe = true;
        await sleep(2500);
        continue;
      }

      if (castProbe && !targetedGuest) {
        await hostPage.evaluate(() => {
          window.dispatchEvent(new CustomEvent("ironsmith:target-choice", {
            detail: { target: { kind: "player", player: 1 } },
          }));
        });
        await sleep(250);
        const submittedTargets = await clickLocalButton(
          hostPage,
          "host-submit-probe-target",
          /SUBMIT TARGETS|SUBMIT/i,
        );
        if (submittedTargets) {
          targetedGuest = true;
          await sleep(2500);
          continue;
        }
      }

      if (castProbe && !paidProbe) {
        const selectedPayment = await clickEnabledButton(
          hostPage,
          "host-select-probe-payment",
          /2 life|PHYREXIAN|\{U\/P\}/i,
        );
        if (selectedPayment) {
          await sleep(250);
        }
        const submittedPayment = await clickLocalButton(
          hostPage,
          "host-submit-probe-payment",
          /SUBMIT|PAY/i,
        );
        if (submittedPayment) {
          paidProbe = true;
          await sleep(3000);
          continue;
        }
      }

      const resolveClick = paidProbe && !guestResolved
        ? await activateLocalButton(guestPage, "guest-resolve-probe", /RESOLVE/i)
        : null;
      if (resolveClick) {
        try {
          await waitForVisibleBodyText(
            hostPage,
            /GY\s*1[\s\S]*DECK\s*52/i,
            "expected Gitaxian Probe to resolve before checking the looked-at hand",
            45000,
          );
        } catch (err) {
          throw new Error(`${err.message}
resolve click: ${JSON.stringify(resolveClick, null, 2)}
host buttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}
guest buttons: ${JSON.stringify(await buttonDebugText(guestPage), null, 2)}
host body:
${await visibleBodyText(hostPage)}
guest body:
${await visibleBodyText(guestPage)}
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-120), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-120), null, 2)}`);
        }
        guestResolved = true;
        let revealText = "";
        try {
          revealText = await waitForVisibleBodyText(
            hostPage,
            /LOOK[\s\S]*Look at target player's hand[\s\S]*Lightning Bolt/i,
            "expected Gitaxian Probe to show the targeted player's hand to its caster",
            30000,
          );
        } catch (err) {
          err.message = `${err.message}\nhost console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-120), null, 2)}\nguest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-120), null, 2)}`;
          throw err;
        }
        assertNoSyncFailureText(revealText, "Gitaxian Probe hand view should stay synced");
        break;
      }

      if (!castProbe) {
        const hostAdvanced = await clickLocalButton(hostPage, "host-setup-probe", /KEEP HAND|PREGAME|UPKEEP|DRAW|PASS PRIORITY|RESOLVE/i);
        if (hostAdvanced) {
          await sleep(3500);
          continue;
        }
        const guestAdvanced = await clickLocalButton(guestPage, "guest-setup-probe", /KEEP HAND|PREGAME|UPKEEP|DRAW|PASS PRIORITY|RESOLVE/i);
        if (guestAdvanced) {
          await sleep(3500);
          continue;
        }
      }

      await sleep(1000);
    }

    assert.equal(castProbe, true, `expected host to cast Gitaxian Probe\n${lastCombinedText}`);
    assert.equal(targetedGuest, true, `expected host to target the other player with Gitaxian Probe\n${lastCombinedText}`);
    assert.equal(paidProbe, true, `expected host to pay for Gitaxian Probe\n${lastCombinedText}`);
    assert.equal(guestResolved, true, `expected guest to resolve Gitaxian Probe\n${lastCombinedText}`);
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS cancelling an in-progress cast stays synced", { timeout: 240000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const hostDeck = deckUrlParam("60 Gitaxian Probe");
    const guestDeck = deckUrlParam("60 Lightning Bolt");
    hostPage = await openFullUiPage(hostContext, `${baseUrl}/?name=Chiplis&deck=${hostDeck}`, "host-cancel-ui");
    await hostPage.getByText("CREATE LOBBY").first().waitFor({ timeout: 30000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).first().click();
    await hostPage.getByText("Host or join").waitFor({ timeout: 10000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).last().click();
    await hostPage.getByText("Share this code").waitFor({ timeout: 40000 });

    const lobbyCode = (await visibleBodyText(hostPage)).match(
      /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i
    )?.[0];
    assert.ok(lobbyCode, "expected the full UI to create a cancel-cast lobby code");

    guestPage = await openFullUiPage(
      guestContext,
      `${baseUrl}/?lobby=${encodeURIComponent(lobbyCode)}&name=Alice&deck=${guestDeck}`,
      "guest-cancel-ui",
    );
    await hostPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });
    await guestPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });

    await hostPage.getByRole("button").filter({ hasText: /START GAME/i }).click();
    await hostPage.getByRole("button").filter({ hasText: /START GAME/i }).waitFor({
      state: "detached",
      timeout: 60000,
    }).catch(() => {});
    await sleep(8000);
    await Promise.all([
      hostPage.keyboard.press("Escape").catch(() => {}),
      guestPage.keyboard.press("Escape").catch(() => {}),
    ]);
    await sleep(500);

    const hostDecisionKind = async () =>
      hostPage.evaluate(() => String(window.__ironsmithE2E?.snapshot?.()?.state?.decision?.kind || ""));

    let castProbe = false;
    let castInProgress = false;
    let cancelled = false;
    let lastCombinedText = "";

    for (let step = 0; step < 80 && !cancelled; step += 1) {
      const hostText = await visibleBodyText(hostPage);
      const guestText = await visibleBodyText(guestPage);
      const combinedText = `${hostText}\n${guestText}`;
      lastCombinedText = combinedText;
      assertNoSyncFailureText(combinedText, "Cancelling an in-progress cast should stay synced");
      assert.doesNotMatch(
        combinedText,
        /Cheat detected|unknown variant `cancel_decision`|does not match pending decision|invalid command payload/i,
        `Cancelling an in-progress cast should not trip cheat detection or a payload error
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-40), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-40), null, 2)}`
      );

      // Once Gitaxian Probe is mid-cast (the caster faces its target/payment
      // decision before the spell is on the stack), cancel it and confirm both
      // peers roll back in sync.
      if (castProbe && !castInProgress) {
        const kind = await hostDecisionKind();
        if (kind === "targets" || /STACK[\s\S]*Gitaxian Probe/i.test(hostText)) {
          castInProgress = true;
          await hostPage.evaluate(() => window.__ironsmithE2E?.cancelDecision?.());
          await sleep(4000);
          cancelled = true;
          break;
        }
      }

      if (!castProbe && await clickLocalButton(hostPage, "host-cast-cancel", /GITAXIAN PROBE/i)) {
        castProbe = true;
        await sleep(2500);
        continue;
      }

      if (!castProbe) {
        const hostAdvanced = await clickLocalButton(hostPage, "host-setup-cancel", /KEEP HAND|PREGAME|UPKEEP|DRAW|PASS PRIORITY|RESOLVE/i);
        if (hostAdvanced) {
          await sleep(3500);
          continue;
        }
        const guestAdvanced = await clickLocalButton(guestPage, "guest-setup-cancel", /KEEP HAND|PREGAME|UPKEEP|DRAW|PASS PRIORITY|RESOLVE/i);
        if (guestAdvanced) {
          await sleep(3500);
          continue;
        }
      }

      await sleep(1000);
    }

    assert.equal(castProbe, true, `expected host to cast Gitaxian Probe\n${lastCombinedText}`);
    assert.equal(castInProgress, true, `expected Gitaxian Probe to reach a cancelable mid-cast decision\n${lastCombinedText}`);

    // After the synced cancel both peers must agree: the spell is off the stack
    // and back in the caster's hand, with no sync failure or cheat flag.
    const settledHost = await visibleBodyText(hostPage);
    const settledGuest = await visibleBodyText(guestPage);
    const settledText = `${settledHost}\n${settledGuest}`;
    assertNoSyncFailureText(settledText, "Cancelled cast should leave both peers synced");
    assert.doesNotMatch(
      settledText,
      /Cheat detected|unknown variant `cancel_decision`|does not match pending decision|invalid command payload/i,
      `Cancelled cast should not trip cheat detection
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-60), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-60), null, 2)}`
    );
    const stackHasProbe = await hostPage.evaluate(() =>
      ((window.__ironsmithE2E?.snapshot?.()?.state?.stack_preview) || [])
        .some((entry) => /Gitaxian Probe/i.test(JSON.stringify(entry)))
    );
    assert.equal(stackHasProbe, false, `expected Gitaxian Probe off the stack after cancel\n${settledText}`);
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS Mishra's Bauble shows the targeted player's library top card to its controller", { timeout: 300000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    ({ hostPage, guestPage } = await startFullUiPeerMatch({
      baseUrl,
      hostContext,
      guestContext,
      hostDeckText: "60 Mishra's Bauble",
      guestDeckText: "60 Lightning Bolt",
      hostLabel: "host-bauble-ui",
      guestLabel: "guest-bauble-ui",
      securityMode: "verified",
    }));
    await sleep(500);

    let castBauble = false;
    let baubleResolved = false;
    let activatedBauble = false;
    let targetedGuest = false;
    let sawLibraryTopCard = false;
    let lastCombinedText = "";

    for (let step = 0; step < 90 && !sawLibraryTopCard; step += 1) {
      const hostText = await visibleBodyText(hostPage);
      const guestText = await visibleBodyText(guestPage);
      const combinedText = `${hostText}\n${guestText}`;
      lastCombinedText = combinedText;
      assertNoSyncFailureText(combinedText, "Mishra's Bauble opponent-library look should stay synced");
      assert.doesNotMatch(
        combinedText,
        /Ziffle reveal-token request is not authorized|Missing audit material for private_view_window|Cheat detected|Invalid priority action ref|Action order mismatch/i,
        `Mishra's Bauble opponent-library look should authorize its private view window
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-40), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-40), null, 2)}`
      );

      if (!castBauble) {
        const result = await clickLocalButton(hostPage, "host-cast-bauble", /MISHRA.S BAUBLE/i);
        if (result) {
          castBauble = true;
          await sleep(2500);
          continue;
        }
      }

      if (castBauble && !baubleResolved) {
        if (/BF\s*1/i.test(hostText) && /Mishra's Bauble[\s\S]*Battlefield/i.test(hostText)) {
          baubleResolved = true;
          continue;
        }
        const hostPass = await clickLocalButton(hostPage, "host-pass-bauble", /PASS PRIORITY|RESOLVE/i);
        if (hostPass) {
          await sleep(2500);
          continue;
        }
        const guestPass = await clickLocalButton(guestPage, "guest-resolve-bauble", /PASS PRIORITY|RESOLVE/i);
        if (guestPass) {
          await sleep(2500);
          continue;
        }
      }

      if (baubleResolved && !activatedBauble) {
        const result = await clickLocalButton(hostPage, "host-activate-bauble", /Look at the top card/i);
        if (result) {
          activatedBauble = true;
          await sleep(2500);
          continue;
        }
        await sleep(1000);
        continue;
      }

      if (activatedBauble && !targetedGuest) {
        await hostPage.evaluate(() => {
          window.dispatchEvent(new CustomEvent("ironsmith:target-choice", {
            detail: { target: { kind: "player", player: 1 } },
          }));
        });
        await sleep(250);
        const submittedTargets = await clickLocalButton(
          hostPage,
          "host-submit-bauble-target",
          /SUBMIT TARGETS|SUBMIT/i,
        );
        if (submittedTargets) {
          targetedGuest = true;
          await sleep(2500);
          continue;
        }
      }

      if (targetedGuest && !sawLibraryTopCard) {
        if (/Look at cards from the top of a library[\s\S]*Lightning Bolt/i.test(hostText)) {
          sawLibraryTopCard = true;
          break;
        }
        const hostPass = await clickLocalButton(hostPage, "host-pass-bauble-ability", /PASS PRIORITY|RESOLVE/i);
        if (hostPass) {
          await sleep(2500);
          continue;
        }
        const guestResolve = await clickLocalButton(guestPage, "guest-resolve-bauble-ability", /RESOLVE|PASS PRIORITY/i);
        if (guestResolve) {
          await sleep(2500);
          continue;
        }
      }

      if (!castBauble) {
        const hostAdvanced = await clickLocalButton(hostPage, "host-setup-bauble", /KEEP HAND|PREGAME|UPKEEP|DRAW|MAIN|PASS PRIORITY|RESOLVE/i);
        if (hostAdvanced) {
          await sleep(3500);
          continue;
        }
        const guestAdvanced = await clickLocalButton(guestPage, "guest-setup-bauble", /KEEP HAND|PREGAME|UPKEEP|DRAW|MAIN|PASS PRIORITY|RESOLVE/i);
        if (guestAdvanced) {
          await sleep(3500);
          continue;
        }
      }

      await sleep(1000);
    }

    assert.equal(castBauble, true, `expected host to cast Mishra's Bauble\n${lastCombinedText}`);
    assert.equal(baubleResolved, true, `expected Mishra's Bauble to resolve to the battlefield\n${lastCombinedText}`);
    assert.equal(activatedBauble, true, `expected host to activate Mishra's Bauble\n${lastCombinedText}`);
    assert.equal(targetedGuest, true, `expected host to target the other player with Mishra's Bauble\n${lastCombinedText}`);
    if (!sawLibraryTopCard) {
      assert.fail(`expected Mishra's Bauble to show the targeted player's library top card to its controller
host buttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}
guest buttons: ${JSON.stringify(await buttonDebugText(guestPage), null, 2)}
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-120), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-120), null, 2)}
last body:
${lastCombinedText}`);
    }
    await sleep(4000);
    const settledText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
    assertNoSyncFailureText(settledText, "Mishra's Bauble look should stay synced after the reveal");
    assert.doesNotMatch(
      settledText,
      /Ziffle reveal-token request is not authorized|Missing audit material for private_view_window/i,
      "Mishra's Bauble private view window audit material should be complete after the reveal",
    );
    const [hostTranscript, guestTranscript] = await Promise.all([
      hostPage.evaluate(() => window.__ironsmithE2E?.auditTranscript?.() || null),
      guestPage.evaluate(() => window.__ironsmithE2E?.auditTranscript?.() || null),
    ]);
    const hostDisclosures = hostTranscript?.privateViewDisclosures || [];
    const guestDisclosures = guestTranscript?.privateViewDisclosures || [];
    assert.ok(
      hostDisclosures.some((disclosure) =>
        Number(disclosure?.owner ?? disclosure?.payload?.owner) === 1
        && Number(disclosure?.viewer ?? disclosure?.payload?.viewer) === 0
        && /Lightning Bolt/i.test(JSON.stringify(disclosure?.payload || {}))
      ),
      `expected the viewer to hold the private-view disclosure for the looked-at card\n${JSON.stringify(hostDisclosures, null, 2)}`,
    );
    assert.equal(
      guestDisclosures.length,
      0,
      "expected the deck owner to never decrypt the looked-at card (mental-poker flow): "
      + JSON.stringify(guestDisclosures, null, 2),
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS guest Mishra's Bauble shows the host's library top card to its controller", { timeout: 300000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    ({ hostPage, guestPage } = await startFullUiPeerMatch({
      baseUrl,
      hostContext,
      guestContext,
      hostDeckText: "60 Lightning Bolt",
      guestDeckText: "60 Mishra's Bauble",
      hostLabel: "host-guest-bauble-ui",
      guestLabel: "guest-guest-bauble-ui",
      securityMode: "verified",
    }));
    await sleep(500);

    let castBauble = false;
    let baubleResolved = false;
    let activatedBauble = false;
    let targetedHost = false;
    let sawLibraryTopCard = false;
    let lastCombinedText = "";

    for (let step = 0; step < 120 && !sawLibraryTopCard; step += 1) {
      const hostText = await visibleBodyText(hostPage);
      const guestText = await visibleBodyText(guestPage);
      const combinedText = `${hostText}\n${guestText}`;
      lastCombinedText = combinedText;
      try {
        assertNoSyncFailureText(combinedText, "guest Mishra's Bauble host-library look should stay synced");
      } catch {
        assert.fail(`guest Mishra's Bauble host-library look should stay synced
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-80), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-80), null, 2)}
body:
${combinedText}`);
      }
      assert.doesNotMatch(
        combinedText,
        /Ziffle reveal-token request is not authorized|Missing audit material for private_view_window|Cheat detected|Invalid priority action ref|Action order mismatch/i,
        `guest Mishra's Bauble host-library look should authorize its private view window
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-40), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-40), null, 2)}`
      );

      if (!castBauble) {
        const result = await clickLocalButton(guestPage, "guest-cast-bauble", /MISHRA.S BAUBLE/i);
        if (result) {
          castBauble = true;
          await sleep(2500);
          continue;
        }
      }

      if (castBauble && !baubleResolved) {
        if (/BF\s*1/i.test(guestText) && /Mishra's Bauble[\s\S]*Battlefield/i.test(guestText)) {
          baubleResolved = true;
          continue;
        }
        const guestPass = await clickLocalButton(guestPage, "guest-pass-bauble", /PASS PRIORITY|RESOLVE/i);
        if (guestPass) {
          await sleep(2500);
          continue;
        }
        const hostPass = await clickLocalButton(hostPage, "host-resolve-bauble", /PASS PRIORITY|RESOLVE/i);
        if (hostPass) {
          await sleep(2500);
          continue;
        }
      }

      if (baubleResolved && !activatedBauble) {
        const result = await clickLocalButton(guestPage, "guest-activate-bauble", /Look at the top card/i);
        if (result) {
          activatedBauble = true;
          await sleep(2500);
          continue;
        }
        await sleep(1000);
        continue;
      }

      if (activatedBauble && !targetedHost) {
        await guestPage.evaluate(() => {
          window.dispatchEvent(new CustomEvent("ironsmith:target-choice", {
            detail: { target: { kind: "player", player: 0 } },
          }));
        });
        await sleep(250);
        const submittedTargets = await clickLocalButton(
          guestPage,
          "guest-submit-bauble-target",
          /SUBMIT TARGETS|SUBMIT/i,
        );
        if (submittedTargets) {
          targetedHost = true;
          await sleep(2500);
          continue;
        }
      }

      if (targetedHost && !sawLibraryTopCard) {
        if (/Look at cards from the top of a library[\s\S]*Lightning Bolt/i.test(guestText)) {
          sawLibraryTopCard = true;
          break;
        }
        const guestPass = await clickLocalButton(guestPage, "guest-pass-bauble-ability", /PASS PRIORITY|RESOLVE/i);
        if (guestPass) {
          await sleep(2500);
          continue;
        }
        const hostResolve = await clickLocalButton(hostPage, "host-resolve-bauble-ability", /RESOLVE|PASS PRIORITY/i);
        if (hostResolve) {
          await sleep(2500);
          continue;
        }
      }

      if (!castBauble) {
        const advancePattern = /KEEP HAND|PREGAME|BEGIN GAME|UPKEEP|DRAW|MAIN|COMBAT|ATTACKERS|BLOCKERS|NO ATTACKERS|DONE|M2|END|CLEAN|PASS PRIORITY|RESOLVE/i;
        const hostAdvanced = await clickLocalButton(hostPage, "host-setup-bauble", advancePattern);
        if (hostAdvanced) {
          await sleep(3500);
          continue;
        }
        const guestAdvanced = await clickLocalButton(guestPage, "guest-setup-bauble", advancePattern);
        if (guestAdvanced) {
          await sleep(3500);
          continue;
        }
      }

      await sleep(1000);
    }

    assert.equal(castBauble, true, `expected guest to cast Mishra's Bauble\n${lastCombinedText}`);
    assert.equal(baubleResolved, true, `expected guest Mishra's Bauble to resolve to the battlefield\n${lastCombinedText}`);
    assert.equal(activatedBauble, true, `expected guest to activate Mishra's Bauble\n${lastCombinedText}`);
    assert.equal(targetedHost, true, `expected guest to target the host with Mishra's Bauble\n${lastCombinedText}`);
    if (!sawLibraryTopCard) {
      assert.fail(`expected guest Mishra's Bauble to show the host's library top card to its controller
host buttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}
guest buttons: ${JSON.stringify(await buttonDebugText(guestPage), null, 2)}
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-120), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-120), null, 2)}
last body:
${lastCombinedText}`);
    }
    await sleep(4000);
    const settledText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
    assertNoSyncFailureText(settledText, "guest Mishra's Bauble look should stay synced after the reveal");
    assert.doesNotMatch(
      settledText,
      /Ziffle reveal-token request is not authorized|Missing audit material for private_view_window/i,
      "guest Mishra's Bauble private view window audit material should be complete after the reveal",
    );
    const [hostTranscript, guestTranscript] = await Promise.all([
      hostPage.evaluate(() => window.__ironsmithE2E?.auditTranscript?.() || null),
      guestPage.evaluate(() => window.__ironsmithE2E?.auditTranscript?.() || null),
    ]);
    const hostDisclosures = hostTranscript?.privateViewDisclosures || [];
    const guestDisclosures = guestTranscript?.privateViewDisclosures || [];
    assert.ok(
      guestDisclosures.some((disclosure) =>
        Number(disclosure?.owner ?? disclosure?.payload?.owner) === 0
        && Number(disclosure?.viewer ?? disclosure?.payload?.viewer) === 1
        && /Lightning Bolt/i.test(JSON.stringify(disclosure?.payload || {}))
      ),
      `expected the guest viewer to hold the private-view disclosure for the looked-at card\n${JSON.stringify(guestDisclosures, null, 2)}`,
    );
    assert.equal(
      hostDisclosures.length,
      0,
      "expected the host deck owner to never decrypt the looked-at card (mental-poker flow): "
      + JSON.stringify(hostDisclosures, null, 2),
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS guest Claws of Gix sacrificing itself stays synced", { timeout: 300000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const match = await startFullUiPeerMatch({
      baseUrl,
      hostContext,
      guestContext,
      hostDeckText: "60 Lightning Bolt",
      guestDeckText: "30 Wastes\n30 Claws of Gix",
      hostName: "Chiplis",
      guestName: "Alice",
      hostLabel: "host-claws-ui",
      guestLabel: "guest-claws-ui",
    });
    hostPage = match.hostPage;
    guestPage = match.guestPage;

    await assertNoFullUiSyncFailures(hostPage, guestPage);
    await waitAndClickLocalButton(hostPage, "host-keep-claws-test", /KEEP HAND/i, 120000);
    await sleep(2500);
    await assertNoFullUiSyncFailures(hostPage, guestPage);
    const guestKeep = await clickLocalButton(guestPage, "guest-keep-claws-test", /KEEP HAND/i);
    if (guestKeep) {
      await sleep(3000);
    }
    await assertNoFullUiSyncFailures(hostPage, guestPage);

    let activatedClaws = false;
    let gainedLife = false;
    let lastCombinedText = "";
    let lastProgress = "start";

    for (let step = 0; step < 140 && !gainedLife; step += 1) {
      const hostText = await visibleBodyText(hostPage);
      const guestText = await visibleBodyText(guestPage);
      const combinedText = `${hostText}\n${guestText}`;
      lastCombinedText = combinedText;
      try {
        assertNoSyncFailureText(combinedText, "guest Claws of Gix activation should stay synced");
        assert.doesNotMatch(
          combinedText,
          /Cheat detected|invalid action sequence|command type does not match pending decision/i,
        );
      } catch {
        const transcriptActions = async (page) => page.evaluate(async () => {
          const transcript = await window.__ironsmithE2E?.auditTranscript?.();
          return (transcript?.actions || []).map((action) => ({
            seq: action.seq,
            actor: action.actor,
            commandType: action.command?.type,
            commandKind: action.command?.kind ?? action.command?.action_ref?.kind ?? null,
          }));
        }).catch((err) => ({ error: String(err?.message || err) }));
        const dispatchEntries = (page) => (page.__peerHarnessConsole || [])
          .filter((entry) => /worker call|synced dispatch:start|synced dispatch:success|synced dispatch:failed|Cheat detected|dry_run_apply_action|action_quorum|apply_action|crypto_material_request:received|crypto_material_request:authorize/.test(String(entry)))
          .slice(-80);
        assert.fail(`guest Claws of Gix activation should stay synced (progress: ${lastProgress})
host transcript: ${JSON.stringify(await transcriptActions(hostPage), null, 2)}
guest transcript: ${JSON.stringify(await transcriptActions(guestPage), null, 2)}
host dispatches: ${JSON.stringify(dispatchEntries(hostPage), null, 2)}
guest dispatches: ${JSON.stringify(dispatchEntries(guestPage), null, 2)}
body:
${combinedText}`);
      }

      // Derive all progress from the guest snapshot instead of latching flags
      // on UI clicks — clicks can race the engine and leave flags stale.
      const board = await guestPage.evaluate(() => {
        const snap = window.__ironsmithE2E?.snapshot?.();
        const state = snap?.state || {};
        const guest = (state.players || []).find((p) => Number(p.id) === 1) || {};
        const battlefield = guest.battlefield || [];
        const claws = battlefield.find((c) => /^Claws of Gix$/i.test(String(c.name || "")));
        return {
          life: Number(guest.life),
          clawsId: claws ? Number(claws.id) : null,
          clawsTapped: claws ? Boolean(claws.tapped) : null,
          hasWastes: battlefield.some((c) => /^Wastes$/i.test(String(c.name || ""))),
          graveyardSize: Number(guest.graveyard_size ?? 0),
          stackNames: (state.stack_preview || []).map((entry) => String(entry?.name || "")),
          decisionKind: String(state.decision?.kind || ""),
          decisionPlayer: state.decision?.player == null ? null : Number(state.decision.player),
          decisionCandidates: (state.decisionCandidates || []).map((entry) => ({
            id: entry.id,
            name: entry.name,
            legal: entry.legal,
          })),
          decisionOptions: (state.decisionOptions || []).map((option) => ({
            index: option.index,
            description: option.description,
            legal: option.legal,
          })),
        };
      });

      if (board.life === 21) {
        gainedLife = true;
        break;
      }

      // Sacrifice cost selection: pick the Claws of Gix itself.
      if (board.decisionKind === "select_objects" && board.decisionPlayer === 1) {
        const candidate = board.decisionCandidates.find((entry) =>
          entry.legal !== false && /^Claws of Gix$/i.test(String(entry.name || ""))
        );
        if (candidate?.id != null) {
          lastProgress = "submitting sacrifice selection";
          await submitFullUiMultiplayerCommand(
            guestPage,
            { type: "select_objects", object_ids: [Number(candidate.id)] },
            "Selected 1 object(s)",
          );
          await sleep(3000);
          continue;
        }
      }

      // Mana payment options for the {1} activation cost.
      if (board.decisionKind === "select_options" && board.decisionPlayer === 1) {
        const legal = board.decisionOptions.filter((option) => option.legal !== false);
        const payment = legal.find((option) =>
          /Wastes|\{C\}|Add/i.test(String(option.description || ""))
        ) ?? legal[0];
        if (payment?.index != null) {
          lastProgress = `submitting payment option ${payment.description}`;
          await guestPage.evaluate(({ index }) => (
            window.__ironsmithE2E.submitMultiplayerCommand(
              { type: "select_options", option_indices: [Number(index)] },
              "Pay Claws of Gix activation",
            )
          ), { index: payment.index });
          await sleep(2500);
          continue;
        }
      }

      // Claws on the battlefield + guest priority: activate it.
      if (
        board.clawsId != null
        && !activatedClaws
        && board.decisionKind === "priority"
        && board.decisionPlayer === 1
      ) {
        lastProgress = "submitting activation";
        await guestPage.evaluate(({ source }) => (
          window.__ironsmithE2E.submitMultiplayerCommand({
            type: "priority_action",
            action_ref: { kind: "activate_ability", source, ability_index: 0 },
          }, "Activate Claws of Gix")
        ), { source: board.clawsId });
        activatedClaws = true;
        await sleep(3000);
        continue;
      }

      // Something is on the stack (the cast or the ability): pass to resolve.
      if (board.stackNames.length > 0) {
        lastProgress = `resolving stack: ${board.stackNames.join(", ")}`;
        const hostPass = await clickLocalButton(hostPage, "host-resolve-claws", /PASS PRIORITY|RESOLVE/i);
        if (hostPass) {
          await sleep(2000);
          continue;
        }
        const guestPass = await clickLocalButton(guestPage, "guest-resolve-claws", /PASS PRIORITY|RESOLVE/i);
        if (guestPass) {
          await sleep(2000);
          continue;
        }
        await sleep(1000);
        continue;
      }

      // Pre-activation setup: get Wastes and Claws onto the guest battlefield.
      if (board.clawsId == null && !activatedClaws) {
        if (!board.hasWastes) {
          const result = await clickLocalButton(guestPage, "guest-play-wastes", /PLAY WASTES/i);
          if (result) {
            lastProgress = "played Wastes";
            await sleep(3000);
            continue;
          }
        } else {
          const result = await clickLocalButton(guestPage, "guest-cast-claws", /CAST CLAWS OF GIX/i);
          if (result) {
            lastProgress = "cast Claws of Gix";
            await sleep(3000);
            continue;
          }
        }
        const advancePattern = /KEEP HAND|PREGAME|BEGIN GAME|UPKEEP|DRAW|MAIN|COMBAT|ATTACKERS|BLOCKERS|NO ATTACKERS|DONE|M2|END|CLEAN|PASS PRIORITY|RESOLVE/i;
        const hostAdvanced = await clickLocalButton(hostPage, "host-setup-claws", advancePattern);
        if (hostAdvanced) {
          await sleep(3000);
          continue;
        }
        const guestAdvanced = await clickLocalButton(guestPage, "guest-setup-claws", advancePattern);
        if (guestAdvanced) {
          await sleep(3000);
          continue;
        }
      }

      // Post-activation: keep passing priority until the ability resolves.
      if (activatedClaws) {
        const guestPass = await clickLocalButton(guestPage, "guest-pass-claws-ability", /PASS PRIORITY|RESOLVE/i);
        if (guestPass) {
          await sleep(2000);
          continue;
        }
        const hostResolve = await clickLocalButton(hostPage, "host-resolve-claws-ability", /RESOLVE|PASS PRIORITY/i);
        if (hostResolve) {
          await sleep(2000);
          continue;
        }
      }

      await sleep(1000);
    }

    const driveFailureDetail = async () => `
progress: ${lastProgress}
host buttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}
guest buttons: ${JSON.stringify(await buttonDebugText(guestPage), null, 2)}
guest decision: ${JSON.stringify((await fullUiSnapshot(guestPage))?.state?.decision || null, null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-60), null, 2)}
last body:
${lastCombinedText}`;
    if (!activatedClaws) {
      assert.fail(`expected guest to activate Claws of Gix\n${await driveFailureDetail()}`);
    }
    if (!gainedLife) {
      assert.fail(`expected guest to gain 1 life from Claws of Gix sacrificing itself
host buttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}
guest buttons: ${JSON.stringify(await buttonDebugText(guestPage), null, 2)}
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-120), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-120), null, 2)}
last body:
${lastCombinedText}`);
    }
    await sleep(4000);
    const settledText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
    assertNoSyncFailureText(settledText, "guest Claws of Gix activation should stay synced after resolution");
    assert.doesNotMatch(
      settledText,
      /Cheat detected|invalid action sequence/i,
      "guest Claws of Gix activation should not trip the anticheat",
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS Urza's Saga chapter III search puts a 0-cost artifact onto the battlefield", { timeout: 420000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const match = await startFullUiPeerMatch({
      baseUrl,
      hostContext,
      guestContext,
      hostDeckText: "20 Urza's Saga\n20 Mishra's Bauble\n20 Mind Stone",
      guestDeckText: "60 Lightning Bolt",
      hostName: "Chiplis",
      guestName: "Alice",
      hostLabel: "host-urzas-saga-ui",
      guestLabel: "guest-urzas-saga-ui",
    });
    hostPage = match.hostPage;
    guestPage = match.guestPage;

    await assertNoFullUiSyncFailures(hostPage, guestPage);
    await waitAndClickLocalButton(hostPage, "host-keep-saga-test", /KEEP HAND/i, 120000);
    await sleep(2500);
    const guestKeep = await clickLocalButton(guestPage, "guest-keep-saga-test", /KEEP HAND/i);
    if (guestKeep) {
      await sleep(3000);
    }
    await assertNoFullUiSyncFailures(hostPage, guestPage);

    let playedSaga = false;
    let sawSearchDecision = false;
    let submittedSearchChoice = false;
    let baubleOnBattlefield = false;
    let searchCandidates = null;
    let lastCombinedText = "";
    let lastProgress = "start";

    for (let step = 0; step < 200 && !baubleOnBattlefield; step += 1) {
      const hostText = await visibleBodyText(hostPage);
      const guestText = await visibleBodyText(guestPage);
      const combinedText = `${hostText}\n${guestText}`;
      lastCombinedText = combinedText;
      try {
        assertNoSyncFailureText(combinedText, "Urza's Saga search should stay synced");
        assert.doesNotMatch(
          combinedText,
          /Cheat detected|hidden object reference is not legal|invalid action sequence/i,
        );
      } catch {
        const dispatchEntries = (page) => (page.__peerHarnessConsole || [])
          .filter((entry) => /worker call|synced dispatch:start|synced dispatch:failed|Cheat detected|apply_action/.test(String(entry)))
          .slice(-120);
        const checkpointDigest = (page) => page.evaluate(async () => {
          const checkpoint = await window.__ironsmithE2E?.publicCheckpoint?.();
          return {
            battlefield: checkpoint?.battlefield,
            stack: (checkpoint?.stack || []).map((entry) => entry.sourceName ?? entry.source_name ?? entry.objectId ?? entry.object_id),
            visibleObjects: (checkpoint?.objects || [])
              .filter((object) => !/library|hand/i.test(String(object.zone || "")))
              .map((object) => ({
                id: object.id,
                name: object.name,
                zone: object.zone,
                hidden: Boolean(object.hiddenCard || object.hidden_card),
              })),
          };
        }).catch((err) => ({ error: String(err?.message || err) }));
        assert.fail(`Urza's Saga search should stay synced (progress: ${lastProgress})
search candidates: ${JSON.stringify(searchCandidates, null, 2)}
host checkpoint: ${JSON.stringify(await checkpointDigest(hostPage), null, 2)}
guest checkpoint: ${JSON.stringify(await checkpointDigest(guestPage), null, 2)}
host dispatches: ${JSON.stringify(dispatchEntries(hostPage), null, 2)}
guest dispatches: ${JSON.stringify(dispatchEntries(guestPage), null, 2)}
body:
${combinedText}`);
      }

      const board = await hostPage.evaluate(() => {
        const snap = window.__ironsmithE2E?.snapshot?.();
        const state = snap?.state || {};
        const hostPlayer = (state.players || []).find((p) => Number(p.id) === 0) || {};
        const battlefield = hostPlayer.battlefield || [];
        return {
          sagaOnBattlefield: battlefield.some((c) => /^Urza's Saga$/i.test(String(c.name || ""))),
          baubleOnBattlefield: battlefield.some((c) => /^Mishra's Bauble$/i.test(String(c.name || ""))),
          decisionKind: String(state.decision?.kind || ""),
          decisionPlayer: state.decision?.player == null ? null : Number(state.decision.player),
          decisionDescription: String(state.decision?.description || ""),
          decisionSource: String(state.decision?.source_name || ""),
          decisionCandidates: (state.decisionCandidates || []).map((entry) => ({
            id: entry.id,
            name: entry.name,
            legal: entry.legal,
          })),
        };
      });

      if (board.baubleOnBattlefield) {
        baubleOnBattlefield = true;
        break;
      }

      // Chapter III search: choose the 0-cost Mishra's Bauble.
      if (
        board.decisionKind === "select_objects"
        && board.decisionPlayer === 0
        && /search|library|urza's saga/i.test(`${board.decisionDescription} ${board.decisionSource}`)
        && !/discard|bottom/i.test(board.decisionDescription)
        && !submittedSearchChoice
      ) {
        sawSearchDecision = true;
        searchCandidates = board.decisionCandidates;
        const candidate = board.decisionCandidates.find((entry) =>
          entry.legal !== false && /^Mishra's Bauble$/i.test(String(entry.name || ""))
        );
        if (candidate?.id != null) {
          lastProgress = "submitting search choice";
          await hostPage.evaluate(({ command }) => (
            window.__ironsmithE2E.submitMultiplayerCommand(command, "Selected 1 object(s)")
          ), { command: { type: "select_objects", object_ids: [Number(candidate.id)] } });
          submittedSearchChoice = true;
          await sleep(4000);
          continue;
        }
        lastProgress = `search decision visible without a legal Bauble candidate: ${JSON.stringify(board.decisionCandidates)}`;
        await sleep(1000);
        continue;
      }

      // Cleanup discards (hand size 8 while goldfishing): discard any card.
      if (
        board.decisionKind === "select_objects"
        && /discard|bottom/i.test(board.decisionDescription)
        && board.decisionPlayer != null
      ) {
        const actorPage = board.decisionPlayer === 0 ? hostPage : guestPage;
        const candidate = await actorPage.evaluate(() => {
          const snap = window.__ironsmithE2E?.snapshot?.();
          const entry = (snap?.state?.decisionCandidates || []).find((c) => c.legal !== false);
          return entry?.id == null ? null : { id: Number(entry.id) };
        });
        if (candidate?.id != null) {
          lastProgress = `discarding for player ${board.decisionPlayer}`;
          await submitFullUiMultiplayerCommand(
            actorPage,
            { type: "select_objects", object_ids: [candidate.id] },
            "Discarded 1 card",
          );
          await sleep(2500);
          continue;
        }
      }

      if (!playedSaga) {
        const result = await clickLocalButton(hostPage, "host-play-saga", /PLAY URZA.S SAGA/i);
        if (result) {
          playedSaga = true;
          lastProgress = "played Urza's Saga";
          await sleep(3000);
          continue;
        }
      }

      // Advance turns until chapter III triggers (two draw steps after entry).
      const advancePattern = /KEEP HAND|PREGAME|BEGIN GAME|UPKEEP|DRAW|MAIN|COMBAT|ATTACKERS|BLOCKERS|NO ATTACKERS|DONE|M2|END|CLEAN|PASS PRIORITY|RESOLVE/i;
      const hostAdvanced = await clickLocalButton(hostPage, "host-advance-saga", advancePattern);
      if (hostAdvanced) {
        await sleep(2000);
        continue;
      }
      const guestAdvanced = await clickLocalButton(guestPage, "guest-advance-saga", advancePattern);
      if (guestAdvanced) {
        await sleep(2000);
        continue;
      }
      await sleep(1000);
    }

    const failureDetail = async () => `
progress: ${lastProgress}
search candidates: ${JSON.stringify(searchCandidates, null, 2)}
host buttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}
guest buttons: ${JSON.stringify(await buttonDebugText(guestPage), null, 2)}
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-80), null, 2)}
last body:
${lastCombinedText}`;
    assert.equal(playedSaga, true, `expected host to play Urza's Saga\n${await failureDetail()}`);
    assert.equal(sawSearchDecision, true, `expected Urza's Saga chapter III search decision\n${await failureDetail()}`);
    assert.equal(submittedSearchChoice, true, `expected to choose Mishra's Bauble from the search\n${await failureDetail()}`);
    if (!baubleOnBattlefield) {
      assert.fail(`expected Mishra's Bauble to be put onto the battlefield\n${await failureDetail()}`);
    }
    await sleep(4000);
    const settledText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
    assertNoSyncFailureText(settledText, "Urza's Saga search should stay synced after resolution");
    assert.doesNotMatch(
      settledText,
      /Cheat detected|hidden object reference is not legal|invalid action sequence/i,
      "Urza's Saga search should not trip the anticheat after resolution",
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS guest Urza's Saga chapter III search puts a 0-cost artifact onto the battlefield", { timeout: 900000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort, {
    VITE_MULTIPLAYER_PLAYER_CLOCK_MS: String(60 * 60 * 1000),
    VITE_ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD: String(15 * 1000),
  });
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const match = await startFullUiPeerMatch({
      baseUrl,
      hostContext,
      guestContext,
      hostDeckText: "60 Lightning Bolt",
      guestDeckText: "20 Urza's Saga\n12 Mishra's Bauble\n28 Lightning Bolt",
      hostName: "Chiplis",
      guestName: "Alice",
      hostLabel: "host-guest-urzas-saga-ui",
      guestLabel: "guest-guest-urzas-saga-ui",
    });
    hostPage = match.hostPage;
    guestPage = match.guestPage;

    await assertNoFullUiSyncFailures(hostPage, guestPage);
    await waitAndClickLocalButton(hostPage, "host-keep-guest-saga-test", /KEEP HAND/i, 120000);
    await sleep(2500);
    const guestMulligan = await clickLocalButton(guestPage, "guest-mulligan-guest-saga-test", /MULLIGAN/i);
    if (guestMulligan) {
      await sleep(6000);
    }
    const guestKeep = await clickLocalButton(guestPage, "guest-keep-guest-saga-test", /KEEP HAND/i);
    if (guestKeep) {
      await sleep(3000);
    }
    await assertNoFullUiSyncFailures(hostPage, guestPage);

    let playedSaga = false;
    let sawSearchDecision = false;
    let submittedSearchChoice = false;
    let baubleOnBattlefield = false;
    let searchCandidates = null;
    let lastCombinedText = "";
    let lastProgress = "start";

    const failureDetail = async () => `
progress: ${lastProgress}
search candidates: ${JSON.stringify(searchCandidates, null, 2)}
host buttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}
guest buttons: ${JSON.stringify(await buttonDebugText(guestPage), null, 2)}
host errors: ${JSON.stringify(hostPage.__peerHarnessErrors || [], null, 2)}
guest errors: ${JSON.stringify(guestPage.__peerHarnessErrors || [], null, 2)}
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-80), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-80), null, 2)}
last body:
${lastCombinedText}`;

    const searchDeadlineAt = Date.now() + 8 * 60 * 1000;
    for (let step = 0; Date.now() < searchDeadlineAt && !baubleOnBattlefield; step += 1) {
      const hostText = await visibleBodyText(hostPage);
      const guestText = await visibleBodyText(guestPage);
      const combinedText = `${hostText}\n${guestText}`;
      lastCombinedText = combinedText;
      try {
        assertNoSyncFailureText(combinedText, "Urza's Saga search should stay synced");
        assert.doesNotMatch(
          combinedText,
          /Cheat detected|hidden object reference is not legal|invalid action sequence/i,
        );
      } catch {
        const dispatchEntries = (page) => (page.__peerHarnessConsole || [])
          .filter((entry) => /worker call|synced dispatch:start|synced dispatch:failed|Cheat detected|apply_action/.test(String(entry)))
          .slice(-120);
        const checkpointDigest = (page) => page.evaluate(async () => {
          const checkpoint = await window.__ironsmithE2E?.publicCheckpoint?.();
          return {
            battlefield: checkpoint?.battlefield,
            stack: (checkpoint?.stack || []).map((entry) => entry.sourceName ?? entry.source_name ?? entry.objectId ?? entry.object_id),
            visibleObjects: (checkpoint?.objects || [])
              .filter((object) => !/library|hand/i.test(String(object.zone || "")))
              .map((object) => ({
                id: object.id,
                name: object.name,
                zone: object.zone,
                hidden: Boolean(object.hiddenCard || object.hidden_card),
              })),
          };
        }).catch((err) => ({ error: String(err?.message || err) }));
        assert.fail(`Urza's Saga search should stay synced (progress: ${lastProgress})
search candidates: ${JSON.stringify(searchCandidates, null, 2)}
host checkpoint: ${JSON.stringify(await checkpointDigest(hostPage), null, 2)}
guest checkpoint: ${JSON.stringify(await checkpointDigest(guestPage), null, 2)}
host dispatches: ${JSON.stringify(dispatchEntries(hostPage), null, 2)}
guest dispatches: ${JSON.stringify(dispatchEntries(guestPage), null, 2)}
body:
${combinedText}`);
      }

      let board = null;
      try {
        board = await withRejectingTimeout(
          guestPage.evaluate(() => {
            const snap = window.__ironsmithE2E?.snapshot?.();
            const state = snap?.state || {};
            const sagaPlayer = (state.players || []).find((p) => Number(p.id) === 1) || {};
            const battlefield = sagaPlayer.battlefield || [];
            return {
              sagaOnBattlefield: battlefield.some((c) => /^Urza's Saga$/i.test(String(c.name || ""))),
              baubleOnBattlefield: battlefield.some((c) => /^Mishra's Bauble$/i.test(String(c.name || ""))),
              stackSources: (state.stack || []).map((entry) =>
                String(entry.source_name || entry.sourceName || entry.name || "")
              ),
              decisionKind: String(state.decision?.kind || ""),
              decisionPlayer: state.decision?.player == null ? null : Number(state.decision.player),
              decisionDescription: String(state.decision?.description || ""),
              decisionSource: String(state.decision?.source_name || ""),
              decisionCandidates: (state.decisionCandidates || []).map((entry) => ({
                id: entry.id,
                name: entry.name,
                legal: entry.legal,
              })),
            };
          }),
          30000,
          "guest Urza's Saga board snapshot",
        );
      } catch (err) {
        assert.fail(`failed to read guest Urza's Saga board snapshot: ${String(err?.message || err)}
${await failureDetail()}`);
      }

      if (board.baubleOnBattlefield) {
        baubleOnBattlefield = true;
        break;
      }

      // Chapter III search: choose the 0-cost Mishra's Bauble.
      const searchDecisionContext = [
        board.decisionDescription,
        board.decisionSource,
        board.stackSources.join(" "),
        combinedText,
      ].join(" ");
      if (
        board.decisionKind === "select_objects"
        && board.decisionPlayer === 1
        && /search|library|urza's saga/i.test(searchDecisionContext)
        && !/discard|bottom/i.test(board.decisionDescription)
        && !submittedSearchChoice
      ) {
        sawSearchDecision = true;
        searchCandidates = board.decisionCandidates;
        const candidate = board.decisionCandidates.find((entry) =>
          entry.legal !== false && /^Mishra's Bauble$/i.test(String(entry.name || ""))
        );
        if (candidate?.id != null) {
          lastProgress = "submitting search choice";
          await submitFullUiMultiplayerCommand(
            guestPage,
            { type: "select_objects", object_ids: [Number(candidate.id)] },
            "Selected 1 object(s)",
          );
          submittedSearchChoice = true;
          await sleep(4000);
          continue;
        }
        lastProgress = `search decision visible without a legal Bauble candidate: ${JSON.stringify(board.decisionCandidates)}`;
        await sleep(1000);
        continue;
      }

      // Cleanup discards (hand size 8 while goldfishing): discard any card.
      if (
        board.decisionKind === "select_objects"
        && /discard|bottom/i.test(board.decisionDescription)
        && board.decisionPlayer != null
      ) {
        const actorPage = board.decisionPlayer === 0 ? hostPage : guestPage;
        let candidate = null;
        try {
          candidate = await withRejectingTimeout(
            actorPage.evaluate(() => {
              const snap = window.__ironsmithE2E?.snapshot?.();
              const entry = (snap?.state?.decisionCandidates || []).find((c) => c.legal !== false);
              return entry?.id == null ? null : { id: Number(entry.id) };
            }),
            30000,
            `discard candidate snapshot for player ${board.decisionPlayer}`,
          );
        } catch (err) {
          assert.fail(`failed to read discard candidate snapshot: ${String(err?.message || err)}
${await failureDetail()}`);
        }
        if (candidate?.id != null) {
          lastProgress = `discarding for player ${board.decisionPlayer}`;
          await submitFullUiMultiplayerCommand(
            actorPage,
            { type: "select_objects", object_ids: [candidate.id] },
            "Discarded 1 card",
          );
          await sleep(2500);
          continue;
        }
      }

      if (
        (board.decisionKind === "attackers" || board.decisionKind === "blockers")
        && board.decisionPlayer != null
      ) {
        const actorPage = board.decisionPlayer === 0 ? hostPage : guestPage;
        const commandType = board.decisionKind === "attackers" ? "declare_attackers" : "declare_blockers";
        lastProgress = `declaring no ${board.decisionKind} for player ${board.decisionPlayer}`;
        await submitFullUiMultiplayerCommand(
          actorPage,
          { type: commandType, declarations: [] },
          board.decisionKind === "attackers" ? "Declared 0 attacker(s)" : "Declared 0 blocker(s)",
        );
        await sleep(1000);
        continue;
      }

      if (!playedSaga) {
        const result = await clickLocalButton(guestPage, "guest-play-saga", /PLAY URZA.S SAGA/i);
        if (result) {
          playedSaga = true;
          lastProgress = "played Urza's Saga (guest)";
          await sleep(3000);
          continue;
        }
      }

      // Advance turns until chapter III triggers (two draw steps after entry).
      // Once Saga is on the stack, prefer stack/priority controls over phase
      // controls so the trigger does not sit unresolved while the game drifts.
      const advancePages = board.decisionPlayer === 0
        ? [
          { page: hostPage, label: "host" },
          { page: guestPage, label: "guest" },
        ]
        : [
          { page: guestPage, label: "guest" },
          { page: hostPage, label: "host" },
        ];
      let advanced = null;
      for (const pattern of [
        /RESOLVE|PASS PRIORITY/i,
        /KEEP HAND|PREGAME|BEGIN GAME|UPKEEP|DRAW|MAIN|COMBAT|ATTACKERS|BLOCKERS|NO ATTACKERS|DONE|M2|END|CLEAN/i,
      ]) {
        for (const { page, label } of advancePages) {
          advanced = await clickLocalButton(page, `${label}-advance-guest-saga`, pattern);
          if (advanced) break;
        }
        if (advanced) break;
      }
      if (advanced) {
        lastProgress = `advanced via ${advanced.text}`;
        await sleep(2000);
        continue;
      }
      await sleep(1000);
    }

    assert.equal(playedSaga, true, `expected guest to play Urza's Saga\n${await failureDetail()}`);
    assert.equal(sawSearchDecision, true, `expected Urza's Saga chapter III search decision\n${await failureDetail()}`);
    assert.equal(submittedSearchChoice, true, `expected to choose Mishra's Bauble from the search\n${await failureDetail()}`);
    if (!baubleOnBattlefield) {
      assert.fail(`expected Mishra's Bauble to be put onto the battlefield\n${await failureDetail()}`);
    }
    await sleep(4000);
    const settledText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
    assertNoSyncFailureText(settledText, "Urza's Saga search should stay synced after resolution");
    assert.doesNotMatch(
      settledText,
      /Cheat detected|hidden object reference is not legal|invalid action sequence/i,
      "Urza's Saga search should not trip the anticheat after resolution",
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});


test("full UI PeerJS Tainted Pact resolution reveals choices and stays synced", { timeout: 240000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const hostDeck = deckUrlParam("30 Black Lotus\n30 Tainted Pact");
    const guestDeck = deckUrlParam("60 Lightning Bolt");
    hostPage = await openFullUiPage(hostContext, `${baseUrl}/?name=Chiplis&deck=${hostDeck}`, "host-tainted-pact-ui");
    await hostPage.getByText("CREATE LOBBY").first().waitFor({ timeout: 30000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).first().click();
    await hostPage.getByText("Host or join").waitFor({ timeout: 10000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).last().click();
    await hostPage.getByText("Share this code").waitFor({ timeout: 40000 });

    const lobbyCode = (await visibleBodyText(hostPage)).match(
      /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i
    )?.[0];
    assert.ok(lobbyCode, "expected the full UI to create a Tainted Pact lobby code");

    guestPage = await openFullUiPage(
      guestContext,
      `${baseUrl}/?lobby=${encodeURIComponent(lobbyCode)}&name=Alice&deck=${guestDeck}`,
      "guest-tainted-pact-ui",
    );
    await hostPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });
    await guestPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });

    await hostPage.getByRole("button").filter({ hasText: /START GAME/i }).click();
    await hostPage.getByRole("button").filter({ hasText: /START GAME/i }).waitFor({
      state: "detached",
      timeout: 60000,
    }).catch(() => {});
    await sleep(8000);
    await Promise.all([
      hostPage.keyboard.press("Escape").catch(() => {}),
      guestPage.keyboard.press("Escape").catch(() => {}),
    ]);
    await sleep(500);

    let castLotus = false;
    let lotusResolved = false;
    let castPact = false;
    let pactOnStack = false;
    let lastDebug = "";
    const stackCardCount = async (page, cardName) =>
      page.locator(".stack-card[data-card-name]").evaluateAll((cards, name) =>
        cards.filter((card) => String(card.getAttribute("data-card-name") || "") === name).length,
        cardName,
      );

    for (let step = 0; step < 100 && !pactOnStack; step += 1) {
      const hostText = await visibleBodyText(hostPage);
      const guestText = await visibleBodyText(guestPage);
      const combinedText = `${hostText}\n${guestText}`;
      lastDebug = combinedText;
      assertNoSyncFailureText(combinedText, "Tainted Pact cast/payment should stay synced");
      assert.doesNotMatch(
        combinedText,
        /Cheat detected|Invalid priority action ref|Action order mismatch/i,
        `Tainted Pact cast/payment should not trip cheat detection
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).filter((line) => /Cheat detected|invalid priority/i.test(line)).slice(-20), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).filter((line) => /Cheat detected|invalid priority/i.test(line)).slice(-20), null, 2)}`
      );

      if (!castLotus) {
        const result = await clickLocalButton(hostPage, "host-cast-lotus", /BLACK LOTUS/i);
        if (result) {
          castLotus = true;
          await sleep(2500);
          continue;
        }
      }

      if (castLotus && !lotusResolved) {
        if (/BF\s*1/i.test(hostText) && /Black Lotus[\s\S]*Battlefield/i.test(hostText)) {
          lotusResolved = true;
          continue;
        }
        const hostPass = await clickLocalButton(hostPage, "host-pass-lotus", /PASS PRIORITY|RESOLVE/i);
        if (hostPass) {
          await sleep(2500);
          continue;
        }
        const guestPass = await clickLocalButton(guestPage, "guest-resolve-lotus", /PASS PRIORITY|RESOLVE/i);
        if (guestPass) {
          await sleep(2500);
          continue;
        }
      }

      if (lotusResolved && !castPact) {
        if (
          /Pay [\s\S]*Tainted Pact|CHOOSE OPTION/i.test(hostText)
          || await stackCardCount(hostPage, "Tainted Pact") > 0
          || await stackCardCount(guestPage, "Tainted Pact") > 0
        ) {
          castPact = true;
          continue;
        }
        const result = await clickLocalButton(hostPage, "host-cast-pact", /TAINTED PACT/i);
        if (result) {
          await sleep(2500);
          continue;
        }
      }

      if (castPact) {
        const pactPaymentPending = /Pay [\s\S]*Tainted Pact|CHOOSE OPTION|CHOOSE COLOR|Choose a color|remaining|Use\s+from mana pool/i.test(hostText);
        if (pactPaymentPending) {
          const submittedPayment = await clickEnabledButton(
            hostPage,
            "host-pay-pact-submit",
            /^SUBMIT(?:\s*\(\d+\/\d+\))?$|^PAY$/i,
          );
          if (submittedPayment) {
            await sleep(2500);
            continue;
          }
          const selectedPayment = await clickUnselectedEnabledButton(
            hostPage,
            "host-pay-pact",
            /BLACK LOTUS|Use\s+from mana pool|BLACK|GENERIC|MANA|\{B\}|\{1\}|^CAST$/i,
          );
          if (selectedPayment) {
            await sleep(2500);
            continue;
          }
        }
        if (
          await stackCardCount(hostPage, "Tainted Pact") > 0
          || await stackCardCount(guestPage, "Tainted Pact") > 0
        ) {
          pactOnStack = true;
          break;
        }
      }

      const hostProgress = await clickLocalButton(hostPage, "host-setup-pact", /KEEP HAND|PREGAME|UPKEEP|DRAW|MAIN|PASS PRIORITY|RESOLVE/i);
      if (hostProgress) {
        await sleep(2500);
        continue;
      }

      const guestProgress = await clickLocalButton(guestPage, "guest-setup-pact", /KEEP HAND|PREGAME|UPKEEP|DRAW|MAIN|PASS PRIORITY|RESOLVE/i);
      if (guestProgress) {
        await sleep(2500);
        continue;
      }

      await sleep(1000);
    }

    assert.equal(castLotus, true, `expected to cast Black Lotus\n${lastDebug}`);
    assert.equal(lotusResolved, true, `expected Black Lotus to resolve\n${lastDebug}`);
    assert.equal(castPact, true, `expected to cast Tainted Pact\n${lastDebug}`);
    assert.equal(pactOnStack, true, `expected Tainted Pact to reach the stack\n${lastDebug}`);
    assertNoSyncFailureText(await visibleBodyText(hostPage), "host should remain synced after Tainted Pact payment");
    assertNoSyncFailureText(await visibleBodyText(guestPage), "guest should remain synced after Tainted Pact payment");

    let sawTaintedPactPrompt = false;
    for (let step = 0; step < 40 && !sawTaintedPactPrompt; step += 1) {
      const hostText = await visibleBodyText(hostPage);
      const guestText = await visibleBodyText(guestPage);
      const combinedText = `${hostText}\n${guestText}`;
      assertNoSyncFailureText(combinedText, "Tainted Pact resolution should stay synced");
      assert.doesNotMatch(
        combinedText,
        /Cheat detected|Invalid priority action ref|Action order mismatch/i,
        `Tainted Pact resolution should not trip cheat detection
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-40), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-40), null, 2)}`
      );
      assert.doesNotMatch(hostText, /put Hidden Card into your hand/i);
      if (/put (Black Lotus|Island|Swamp|Tainted Pact) into your hand/i.test(hostText)) {
        sawTaintedPactPrompt = true;
        break;
      }
      const hostPass = await clickLocalButton(hostPage, "host-pass-pact", /PASS PRIORITY|RESOLVE/i);
      if (hostPass) {
        await sleep(2500);
        continue;
      }
      const guestResolve = await clickLocalButton(guestPage, "guest-resolve-pact", /RESOLVE|PASS PRIORITY/i);
      if (guestResolve) {
        await sleep(2500);
        continue;
      }
      await sleep(1000);
    }
    assert.equal(sawTaintedPactPrompt, true, `expected Tainted Pact to reveal a named card choice\n${await visibleBodyText(hostPage)}`);
    await sleep(4000);
    {
      const hostText = await visibleBodyText(hostPage);
      const guestText = await visibleBodyText(guestPage);
      const combinedText = `${hostText}\n${guestText}`;
      assertNoSyncFailureText(combinedText, "Tainted Pact first choice should not fail while waiting for input");
      assert.doesNotMatch(hostText, /put Hidden Card into your hand/i);
      assert.doesNotMatch(
        combinedText,
        /Missing audit material for public_view_window|Missing public_open audit opening|Cheat detected|Invalid priority action ref|Action order mismatch/i,
        `Tainted Pact first choice should include public-view audit material
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-40), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-40), null, 2)}`,
      );
    }

    let sawDuplicateStopReveal = false;
    let declinedPactCards = 0;
    let lastDuplicateStopText = "";
    for (let step = 0; step < 50 && !sawDuplicateStopReveal; step += 1) {
      const hostText = await visibleBodyText(hostPage);
      const guestText = await visibleBodyText(guestPage);
      const combinedText = `${hostText}\n${guestText}`;
      lastDuplicateStopText = hostText;
      assertNoSyncFailureText(combinedText, "Tainted Pact duplicate-stop reveal should stay synced");
      assert.doesNotMatch(
        combinedText,
        /Missing public_open audit opening|Cheat detected|Invalid priority action ref|Action order mismatch/i,
        `Tainted Pact duplicate-stop reveal should not trip audit verification
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-40), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-40), null, 2)}`
      );
      if (
        declinedPactCards > 0
        && /EXL\s*[2-9]/i.test(hostText)
        && /(Reveal exiled library cards|Deck\s*->\s*Exile)/i.test(hostText)
        && /(Island|Swamp|Black Lotus|Tainted Pact)/i.test(hostText)
        && !/Card #\d+/i.test(hostText)
        && !/(Reveal exiled library cards|Deck\s*->\s*Exile)[\s\S]{0,240}Hidden Card/i.test(hostText)
      ) {
        sawDuplicateStopReveal = true;
        break;
      }
      if (/Put .* into your hand\?/i.test(hostText) && /(?:^|\n)NO(?:\n|$)/i.test(hostText)) {
        const declinedCard = await clickEnabledButton(
          hostPage,
          `host-decline-pact-card-${declinedPactCards + 1}`,
          /^NO$/i,
        );
        if (declinedCard) {
          declinedPactCards += 1;
          await sleep(2500);
          continue;
        }
      }
      await sleep(1000);
    }
    assert.ok(
      declinedPactCards > 0,
      `expected to decline at least one Tainted Pact card before duplicate stop
${lastDuplicateStopText}`,
    );
    assert.equal(
      sawDuplicateStopReveal,
      true,
      `expected Tainted Pact duplicate-stop card to be revealed instead of Hidden Card
${lastDuplicateStopText}
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-20), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-20), null, 2)}`,
    );
    await hostPage.waitForFunction(
      () => /1\s*\/\s*[2-9]/.test(document.body.innerText || "")
        && /Deck\s*->\s*Exile/i.test(document.body.innerText || ""),
      null,
      { timeout: 5000 },
    );
    assert.match(
      await visibleBodyText(hostPage),
      /1\s*\/\s*[2-9]/i,
      "Tainted Pact reveal inspector should include the exiled cards",
    );
    assert.match(
      await visibleBodyText(hostPage),
      /Deck\s*->\s*Exile/i,
      "Tainted Pact reveal inspector should use the runtime library-to-exile zone label",
    );
    assert.doesNotMatch(
      await visibleBodyText(hostPage),
      /Hidden\s*->\s*Exile/i,
      "Tainted Pact reveal inspector should not fall back to hidden-to-exile labels",
    );

    const completedDuplicateRevealStep =
      await clickLocalButton(hostPage, "host-complete-duplicate-pact-reveal", /^DONE$/i)
      || await clickEnabledButton(hostPage, "host-complete-duplicate-pact-reveal", /^DONE$/i);
    if (completedDuplicateRevealStep) {
      await sleep(1000);
    }

    let sawFinalPactState = false;
    let finalPactText = "";
    for (let step = 0; step < 20 && !sawFinalPactState; step += 1) {
      const hostText = await visibleBodyText(hostPage);
      const guestText = await visibleBodyText(guestPage);
      const combinedText = `${hostText}\n${guestText}`;
      finalPactText = hostText;
      assertNoSyncFailureText(combinedText, "Tainted Pact final state should stay synced");
      assert.doesNotMatch(
        combinedText,
        /Missing public_open audit opening|Cheat detected|Invalid priority action ref|Action order mismatch/i,
        `Tainted Pact final state should not trip audit verification
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-40), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-40), null, 2)}`
      );
      assert.doesNotMatch(hostText, /Card #\d+/i, `Tainted Pact should not leave stale viewed-card labels\n${hostText}`);
      if (/EXL\s*[2-9]/i.test(hostText) && /GY\s*[1-9]/i.test(hostText)) {
        sawFinalPactState = true;
        break;
      }
      await sleep(1000);
    }
    assert.equal(
      sawFinalPactState,
      true,
      `expected declined duplicate-stop Tainted Pact flow to leave two cards in exile and Tainted Pact in graveyard\n${finalPactText}`,
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});
