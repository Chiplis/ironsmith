import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

async function promptText(page, port, query) {
  await page.goto(`http://127.0.0.1:${port}/tests/decision-prompt-locale.html?${query}`);
  const probe = page.locator("[data-decision-summary-probe]");
  await probe.waitFor();
  await page.waitForTimeout(600);
  // Keyword helper spans may add layout whitespace to innerText.
  return (await probe.innerText()).replace(/\s+/g, " ").trim();
}

test("a decision prompt follows the source card's localized printing", async () => {
  const vite = await createServer({server:{host:"127.0.0.1",port:0},logLevel:"silent"});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const port = vite.httpServer.address().port;
    const errors = [];
    const page = await browser.newPage({viewport:{width:900,height:600}});
    page.on("pageerror", e => errors.push(e.message));

    assert.match(await promptText(page, port, "locale=es"), /Copiar ese hechizo/);
    assert.match(
      await promptText(page, port, "locale=es&description=The%20copy%20targets%20Ivy"),
      /La copia hace objetivo a Ivy/
    );
    // A prompt the printed text cannot account for stays in English.
    assert.match(await promptText(page, port, "locale=es&description=Perform%20the%20effect"), /Perform the effect/);
    assert.match(await promptText(page, port, "locale=en"), /Copy that spell/);

    // Yawgmoth's activated ability: an engine-composed frame around a card
    // name, and cost options quoted from the printed cost clause.
    const costPrompt = "source=8&description=Choose%20the%20next%20cost%20to%20pay%20for%20Yawgmoth%2C%20Thran%20Physician's%20ability";
    assert.match(
      await promptText(page, port, `locale=es&${costPrompt}`),
      /Elige el siguiente coste a pagar de la habilidad de Yawgmoth/
    );
    assert.match(await promptText(page, port, "locale=es&source=8&description=Pay%201%20life"), /Pagar 1 vida/);
    assert.match(
      await promptText(page, port, "locale=es&source=8&description=Sacrifice%20another%20creature"),
      /Sacrificar otra criatura/
    );

    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
