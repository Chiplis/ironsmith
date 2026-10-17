// Probe: Peek at Bob's hand, then check the hand fan shows those cards face up
// after the look ends; Opt (scry 1) leaves an eye beside Alice's deck.
import { chromium } from 'playwright';
import { encodePuzzlePayload } from '../src/lib/puzzles.js';
const out = process.argv[2];
const PUZZLE = encodePuzzlePayload({ version: 1, players: [
  { name: 'Alice', life: 20, zones: { battlefield: ['Island', 'Island', 'Island', 'Island'], hand: ['Peek', 'Opt'],
    library: ['Plains', 'Plains', 'Plains', 'Plains', 'Plains', 'Plains', 'Forest', 'Mountain'] } },
  { name: 'Bob', life: 20, zones: { battlefield: ['Grizzly Bears'], hand: ['Lightning Bolt', 'Counterspell', 'Shock'],
    library: Array.from({ length: 8 }, () => 'Swamp') } },
] });
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1600, height: 900 } });
page.on('pageerror', (e) => console.log('PAGEERROR', e.message));
await page.goto(`http://localhost:5174/?puzzle=${PUZZLE}`);
await page.locator('.game-card.hand-card[data-card-name="Peek"]').waitFor({ timeout: 90000 });
await page.keyboard.press('Escape');
for (let step = 0; step < 14; step += 1) {
  const button = page.locator('.decision-main-button').first();
  if (await button.count() === 0) break;
  const label = (await button.textContent() || '').trim();
  if (/^go to combat/i.test(label)) break;
  if (!/^(keep hand|pregame|continue|begin game|go to)/i.test(label)) break;
  await page.keyboard.press('Escape');
  await button.click({ force: true, timeout: 5000 }).catch((e) => console.log('click fail', label, e.message.split('\n')[0]));
  await page.waitForTimeout(1500);
}
const shot = (name) => page.screenshot({ path: `${out}/${name}.png` });
const buttons = async () => (await page.locator('button:visible').allTextContents()).map(s => s.trim()).filter(Boolean).slice(0, 60);
async function cast(name) {
  const card = page.locator(`.game-card.hand-card[data-card-name="${name}"]`);
  await card.click({ force: true, timeout: 10000 }).catch((e) => console.log('card click', e.message.split('\n')[0]));
  await page.waitForTimeout(1500);
  console.log('menu', await buttons());
  const castButton = page.getByRole('button', { name: new RegExp(`^Cast ${name}`, 'i') }).first();
  if (await castButton.count()) await castButton.click({ force: true, timeout: 10000 }).catch(() => {});
  await page.waitForTimeout(2000);
}
async function clickButton(re) {
  const b = page.getByRole('button', { name: re }).first();
  if (await b.count()) { await b.click(); await page.waitForTimeout(1200); return true; }
  return false;
}
const state = () => page.evaluate(() => window.__ironsmithLastSnapshot || null);
await cast('Peek');
await shot('1-peek-cast');
console.log('after cast', await buttons());
// Target Bob.
const bob = page.locator('[data-player-target-name]').first();
if (await bob.count()) { await bob.click(); await page.waitForTimeout(1200); }
await shot('2-peek-target');
console.log('after target', await buttons());
for (let i = 0; i < 6; i += 1) {
  if (await clickButton(/^(submit|confirm|pay|auto|done|ok|resolve|pass)/i)) continue;
  break;
}
await shot('3-peek-resolving');
console.log('after resolve attempts', await buttons());
await browser.close();
