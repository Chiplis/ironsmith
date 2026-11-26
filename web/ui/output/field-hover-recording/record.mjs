import fs from 'node:fs'; import path from 'node:path';
const { chromium } = await import('/Users/chiplis/ironsmith/web/ui/node_modules/playwright/index.mjs');
const out = process.argv[2]; const url = process.argv[3] || 'http://localhost:5174/'; const name = process.argv[4] || 'Omnath';
fs.rmSync(out, { recursive: true, force: true }); fs.mkdirSync(path.join(out, 'frames'), { recursive: true });
const browser = await chromium.launch({ headless: true, channel: 'chromium', args: ['--use-angle=metal', '--enable-gpu', '--ignore-gpu-blocklist'] });
const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 }, recordVideo: { dir: out, size: { width: 1280, height: 800 } } });
const page = await ctx.newPage();
await page.goto(url);
await page.waitForFunction(() => document.querySelectorAll('.game-card.field-card').length >= 6, null, { timeout: 180000 });
await page.waitForTimeout(4000);
for (let i = 0; i < 3; i++) { await page.keyboard.press('Escape'); await page.waitForTimeout(400); }
await page.screenshot({ path: path.join(out, 'start.png') });
const box = await page.evaluate((name) => { const all = [...document.querySelectorAll('.game-card.field-card')].map(e => [e, e.getBoundingClientRect()]).filter(([e, r]) => r.y > 150 && r.y + r.height < 700 && r.x > 150 && r.x + r.width < 1000 && !e.textContent.includes('x')); all.sort((p, q) => Math.abs(p[1].y - 450) - Math.abs(q[1].y - 450)); const cs = all.map(p => p[0]); const c = cs.find(e => e.textContent.includes(name)) || cs[0]; const r = c.getBoundingClientRect(); return { x: r.x, y: r.y, w: r.width, h: r.height, label: c.textContent.slice(0, 30), all: cs.length }; }, name);
await page.mouse.move(1150, 330); await page.waitForTimeout(1500);
await page.evaluate(() => { window.__lo = []; new PerformanceObserver(l => { for (const e of l.getEntries()) window.__lo.push([Math.round(performance.timeOrigin + e.startTime), Math.round(e.duration), (e.scripts || []).map(s => [s.invoker.slice(0, 40), Math.round(s.duration), s.sourceFunctionName])]); }).observe({ type: 'long-animation-frame' }); });
const cdp = await ctx.newCDPSession(page);
const frames = []; let n = 0;
cdp.on('Page.screencastFrame', (f) => { cdp.send('Page.screencastFrameAck', { sessionId: f.sessionId }).catch(() => {}); const i = n++; frames.push({ i, ts: f.metadata.timestamp }); fs.writeFileSync(path.join(out, 'frames', String(i).padStart(4, '0') + '.jpg'), Buffer.from(f.data, 'base64')); });
await cdp.send('Page.startScreencast', { format: 'jpeg', quality: 95, everyNthFrame: 1 });
await page.waitForTimeout(400);
const events = [];
for (let k = 0; k < 2; k++) {
  events.push({ type: 'enter', ts: Date.now() / 1000 });
  await page.mouse.move(box.x + box.w / 2, box.y + box.h / 2);
  await page.waitForTimeout(1500);
  events.push({ type: 'leave', ts: Date.now() / 1000 });
  await page.mouse.move(1150, 330);
  await page.waitForTimeout(1500);
}
await cdp.send('Page.stopScreencast');
const loaf = await page.evaluate(() => window.__lo);
const gl = await page.evaluate(() => { const g = document.createElement('canvas').getContext('webgl'); const e = g && g.getExtension('WEBGL_debug_renderer_info'); return e ? g.getParameter(e.UNMASKED_RENDERER_WEBGL) : 'none'; });
fs.writeFileSync(path.join(out, 'meta.json'), JSON.stringify({ box, frames, events, loaf, gl }));
await ctx.close(); await browser.close();
console.log('frames', n, JSON.stringify(box));
