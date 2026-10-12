import { readFile, writeFile, mkdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const donors = JSON.parse(await readFile(path.join(root, 'scripts/fixtures/default-card-frame-donors.json'), 'utf8'));
const cachedMasks = process.env.DEFAULT_FRAME_MASKS
  ? JSON.parse(await readFile(process.env.DEFAULT_FRAME_MASKS, 'utf8')) : [];
const vite = await createServer({ root, configFile: false, resolve: { alias: { '@': path.join(root, 'src') } }, server: { host: '127.0.0.1', port: 0 }, logLevel: 'error' });
await vite.listen();
let browser;
try {
  browser = await chromium.launch();
  const page = await browser.newPage();
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/default-card-frames.html`);
  const catalog = {};
  const output = path.join(root, 'public/card-frames/default');
  await mkdir(output, { recursive: true });
  for (const donor of donors) {
    const cached = cachedMasks.find(entry => entry.key === donor.key);
    const result = await page.evaluate(async ({ donor, cached }) => {
      const { defaultCardFrameTemplate } = await import('/src/lib/default-card-frame-template.js');
      const { sampleCardFrameColors } = await import('/src/lib/card-frame-processing.js');
      const { cardTypography } = await import('/src/lib/card-typography.js');
      const typography = cached?.typography || cardTypography(donor.printing);
      await Promise.all(['title', 'type', 'rules', 'stats'].map(k => document.fonts.load(`${k === 'rules' ? 400 : k === 'stats' ? typography.style['--card-stats-weight'] : typography.titleWeight} 100px ${typography[k]}`)));
      const style = cached?.style || await sampleCardFrameColors(donor.printing.image_uris.normal, { printing: donor.printing, typography });
      return { ...await defaultCardFrameTemplate(style), typography: { ...cardTypography({ ...donor.printing, frame: '2015' }), printingReady: true } };
    }, { donor, cached });
    await writeFile(path.join(output, `${donor.key}.webp`), Buffer.from(result.image.split(',')[1], 'base64'));
    catalog[donor.key] = { donor: { name: donor.printing.name, id: donor.printing.id, url: donor.printing.scryfall_uri },
      style: result.style, typography: result.typography };
    console.log(`${donor.key}: ${donor.printing.name}`);
  }
  await writeFile(path.join(root, 'src/lib/default-card-frames.generated.json'), JSON.stringify(catalog, null, 2) + '\n');
  // Known nonstandard layouts must retain their existing renderer even before
  // printing metadata arrives. Include face names for multi-face objects.
  const cards = JSON.parse(await readFile(path.join(root, '../../cards.json'), 'utf8'));
  const exceptions = [...new Set(cards.filter(c => c.layout !== 'normal').flatMap(c => [c.name, ...(c.card_faces || []).map(f => f.name)]).map(n => n.toLowerCase()))].sort();
  await writeFile(path.join(root, 'src/lib/nonstandard-card-layouts.generated.json'), JSON.stringify(exceptions) + '\n');
} finally { await browser?.close(); await vite.close(); }
