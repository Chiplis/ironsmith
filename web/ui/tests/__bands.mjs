import { chromium } from "playwright";
const browser = await chromium.launch();
const page = await browser.newPage();
await page.goto("about:blank");
const result = await page.evaluate(async ({ cards }) => {
  const luminance = ([r, g, b]) => 0.2126 * (r / 255) + 0.7152 * (g / 255) + 0.0722 * (b / 255);
  const out = {};
  for (const [name, { url, box }] of Object.entries(cards)) {
    const img = new Image();
    img.crossOrigin = "anonymous";
    await new Promise((res, rej) => { img.onload = res; img.onerror = rej; img.src = url; });
    const c = document.createElement("canvas");
    c.width = img.naturalWidth; c.height = img.naturalHeight;
    const ctx = c.getContext("2d", { willReadFrequently: true });
    ctx.drawImage(img, 0, 0);
    const x = Math.ceil(box.x + 9), y = Math.ceil(box.y + 8);
    const scan = ctx.getImageData(x, y, Math.floor(box.width - 18), Math.floor(box.height - 16));
    // materialColor approximation: most common luminance bucket = paper
    const counts = new Map();
    for (let p = 0; p < scan.width * scan.height; p++) {
      const v = Math.round(luminance([scan.data[p*4], scan.data[p*4+1], scan.data[p*4+2]]) * 20);
      counts.set(v, (counts.get(v) || 0) + 1);
    }
    const paper = [...counts.entries()].sort((a, b) => b[1] - a[1])[0][0] / 20;
    const ink = new Uint8Array(scan.width * scan.height); const rows = [];
    for (let py = 0; py < scan.height; py++) {
      let count = 0;
      for (let px = 0; px < scan.width; px++) {
        const p = py * scan.width + px;
        const rgb = [scan.data[p*4], scan.data[p*4+1], scan.data[p*4+2]];
        const value = luminance(rgb);
        const printedInk = paper < .2
          ? Math.min(...rgb) > 170 && Math.max(...rgb) - Math.min(...rgb) < 65 && value > paper
          : value < paper;
        if (printedInk && (Math.max(paper, value) + .05) / (Math.min(paper, value) + .05) > 2.5) { ink[p] = 1; count++; }
      }
      if (count >= 3) rows.push(py);
    }
    const bands = [];
    for (const row of rows) {
      const band = bands.at(-1);
      if (!band || row - band.bottom > 2) bands.push({ top: row, bottom: row }); else band.bottom = row;
    }
    const profile = [];
    for (let py = 0; py < scan.height; py++) {
      let count = 0, left = scan.width, right = 0;
      for (let px = 0; px < scan.width; px++) if (ink[py * scan.width + px]) { count++; left = Math.min(left, px); right = Math.max(right, px); }
      profile.push({ y: py + y, count, w: count ? right - left + 1 : 0 });
    }
    out[name + "_profile"] = profile.filter(r => r.y >= 425 && r.y <= 480);
    out[name] = { paper: +paper.toFixed(3), scanW: scan.width, bands: bands.map(b => {
      let left = scan.width, right = 0;
      for (let py = b.top; py <= b.bottom; py++) for (let px = 0; px < scan.width; px++)
        if (ink[py * scan.width + px]) { left = Math.min(left, px); right = Math.max(right, px); }
      return { top: b.top + y, h: b.bottom - b.top + 1, w: right - left + 1, kept: (b.bottom - b.top + 1) >= 7 && (b.bottom - b.top + 1) <= 32 };
    }) };
  }
  return out;
}, { cards: {
  asc: { url: "https://cards.scryfall.io/normal/front/0/3/03f029ff-b296-422e-a9e1-db0a5d27a2e8.jpg", box: { x: 28, y: 418, width: 423, height: 215 } },
  yawg: { url: "https://cards.scryfall.io/normal/front/b/5/b5a79f5d-d0df-4799-ac3a-84305e3af0c9.jpg", box: { x: 28, y: 418, width: 423, height: 215 } },
} });
for (const [name, data] of Object.entries(result)) {
  if (name.endsWith("_profile")) {
    console.log(`\n--- ${name} (row ink counts) ---`);
    for (const r of data) console.log(`  y=${r.y} count=${r.count} w=${r.w}`);
    continue;
  }
  console.log(`\n=== ${name} paper=${data.paper} scanW=${data.scanW} ===`);
  for (const b of data.bands) console.log(`  top=${b.top} h=${b.h} w=${b.w} ${b.kept ? "KEPT" : "dropped"}`);
}
await browser.close();
