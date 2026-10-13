import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { resolve, dirname, basename } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import assert from 'node:assert/strict';

const [fontPath, symbolPath] = process.argv.slice(2);
if (!fontPath) throw new Error('Usage: node scripts/mse-check-fonts.mjs FONT_FILE');
const font = await readFile(resolve(fontPath));
const library = resolve(dirname(fileURLToPath(import.meta.url)), '../src/lib/mse');
const server = createServer(async (req, res) => {
  try {
    if (req.url === '/symbol' && symbolPath) { res.end(await readFile(resolve(symbolPath))); return; }
    if (req.url === '/font') { res.end(font); return; }
    if (req.url === '/') { res.setHeader('Content-Type','text/html'); res.end('<!doctype html><title>MSE font check</title>'); return; }
    if (!/^\/[a-z-]+\.mjs$/.test(req.url)) { res.writeHead(404); res.end(); return; }
    res.setHeader('Content-Type', 'text/javascript');
    res.end(await readFile(resolve(library, basename(req.url))));
  } catch { res.writeHead(500); res.end(); }
});
await new Promise(done => server.listen(0, '127.0.0.1', done));
let browser;
try {
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  await page.goto(`http://127.0.0.1:${server.address().port}`);
  const result = await page.evaluate(async (hasSymbol) => {
    const { MseBrowserFonts } = await import('/browser-fonts.mjs');
    const { measureTextRun } = await import('/text-metrics.mjs');
    const fonts = new MseBrowserFonts();
    await fonts.register('Fixture', 'url("/font")');
    const metadata = await fonts.registerFile(await (await fetch('/font')).arrayBuffer());
    const font = { name: 'Fixture', size: 15, scale_down_to: 8 };
    const automatic = await fonts.prepare({ ...font, name: metadata.family });
    const automaticWidth = automatic.measure('AV Magic',font,1).width;
    const provider = await fonts.prepare(font);
    const measured = measureTextRun('AV Magic\nWUBRG', font, provider);
    const firstLine = measured.characters.slice(0,8).reduce((sum,c) => sum+c.width,0);
    const expected = provider.measure('AV Magic', font, 1).width;
    let missing = '';
    try { await fonts.prepare({ ...font, name: 'Missing' }); } catch (error) { missing = error.code; }
    const { parseTaggedText } = await import('/tagged-text.mjs');
    const { layoutTaggedText } = await import('/tagged-layout.mjs');
    const tagged = await layoutTaggedText(parseTaggedText('Magic <b>bold</b> text wraps here.', font), fonts, null, null, { width: 100, height: 60, multiLine: true, alignment: 257 });
    const { paintTextLayout } = await import('/text-painter.mjs');
    const canvas = document.createElement('canvas'); canvas.width = 160; canvas.height = 100;
    document.body.append(canvas);
    const context = canvas.getContext('2d');
    await paintTextLayout(context, tagged, fonts, { x: 10, y: 10 });
    const paintedPixels = context.getImageData(0,0,160,100).data.filter((v,i) => i%4===3 && v>0).length;
    let symbolPixelsChanged = 0, shadowPixelsChanged = 0;
    if (hasSymbol) {
      const { decodePng } = await import('/png-decoder.mjs');
      const raster = await decodePng(new Uint8Array(await (await fetch('/symbol')).arrayBuffer()));
      const generic = { code: '[0-9]+', regex: /^[0-9]+/, enabled: true, drawText: 0, image: {}, imageSize: 135,
        font: { ...font, size: 1, max_stretch: 0.8 }, alignment: 514, margins: { left: 0.05, right: 0.05, top: 0, bottom: 0 } };
      const symbols = { settings: { size: 30, scale_down_to: 15, alignment: 514 }, font: { name: 'fixture', horizontalSpace: 2, verticalSpace: 1, symbols: [generic] } };
      const renderer = { render: async () => raster };
      const cost = await layoutTaggedText(parseTaggedText('<sym>12</sym>',font), fonts, symbols, renderer, {width: 150,height: 80,multiLine: false,alignment:257});
      context.clearRect(0,0,160,100);
      await paintTextLayout(context,cost,fonts);
      const withText = context.getImageData(0,0,160,100).data;
      cost.measurement.runs[0].measured.glyphs[0].drawText = '';
      context.clearRect(0,0,160,100);
      await paintTextLayout(context,cost,fonts);
      const withoutText = context.getImageData(0,0,160,100).data;
      for (let i=0;i<withText.length;i+=4) if (withText.slice(i,i+4).some((v,j)=>v!==withoutText[i+j])) symbolPixelsChanged++;
      Object.assign(cost.measurement.runs[0].measured.settings, { shadow_color: {channels:[0,0,0,255]}, shadow_blur: 2, shadow_displacement_x: 3, shadow_displacement_y: 3 });
      context.clearRect(0,0,160,100);
      await paintTextLayout(context,cost,fonts);
      const withShadow=context.getImageData(0,0,160,100).data;
      for (let i=0;i<withShadow.length;i+=4) if (withShadow.slice(i,i+4).some((v,j)=>v!==withoutText[i+j])) shadowPixelsChanged++;
    }
    const {createBrowserSymbolRenderer}=await import('/browser-symbols.mjs');
    const symbol=await createBrowserSymbolRenderer()({filename:'',variation:{type:'solid',borderRadius:0.1,fill_color:[255,0,0,255],border_color:[0,0,0,255]}},{width:20,height:20});
    const sample=(x,y)=>Array.from(symbol.data.slice((y*symbol.width+x)*4,(y*symbol.width+x)*4+4));
    const vectorSymbol={width:symbol.width,height:symbol.height,center:sample(50,50),border:sample(17,50),outside:sample(0,0)};
    fonts.dispose();
    return { vectorSymbol, automaticWidth, fontFamily: metadata.family, shadowPixelsChanged, symbolPixelsChanged, firstLine, expected, missing, width: measured.width, height: measured.height, characters: measured.characters.length, paintedPixels, tagged: { fits: tagged.fits, scale: tagged.scale, lines: tagged.lines.length, width: tagged.width, height: tagged.height } };
  }, Boolean(symbolPath));
  if (symbolPath) { assert.ok(result.symbolPixelsChanged > 20); assert.ok(result.shadowPixelsChanged > 20); }
  assert.ok(Math.abs(result.firstLine - result.expected) < 1e-9);
  assert.ok(Math.abs(result.automaticWidth - result.expected) < 1e-9);
  assert.deepEqual(result.vectorSymbol,{width:100,height:100,center:[255,0,0,255],border:[0,0,0,255],outside:[0,0,0,0]});
  assert.equal(result.missing, 'font-missing');
  assert.ok(result.width > 0 && result.height > 0);
  assert.equal(result.characters, 14);
  assert.equal(result.tagged.fits, true);
  assert.ok(result.tagged.lines > 1);
  assert.ok(result.paintedPixels > 100);
  console.log(JSON.stringify({ status: 'passed', font: resolve(fontPath), ...result, nativeParityVerified: false }, null, 2));
} finally { await browser?.close(); await new Promise(done => server.close(done)); }
