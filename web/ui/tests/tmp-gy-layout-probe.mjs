import { chromium } from 'playwright';
import { createServer } from 'vite';
const out = process.argv[2] || '/private/tmp/claude-501/-Users-chiplis-ironsmith/7de5e472-ad8d-4485-83bc-72fe4445929d/scratchpad/gy.png';
const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
await vite.listen();
const browser = await chromium.launch();
try {
  const page = await browser.newPage({viewport:{width:1440,height:900}});
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/`);
  await page.locator('.my-zone-board-shell [data-zone-pile="graveyard"]').waitFor({timeout:60000});
  await page.waitForTimeout(1500);
  await page.evaluate(() => { const d=document.querySelector('.my-zone-board-shell .exile-chat-dock'); d.innerHTML='<section class="lobby-chat" data-collapsed="false"><button class="lobby-chat-header">Lobby chat<span>▾</span></button><div class="lobby-chat-body"><div class="lobby-chat-body-inner"><div class="lobby-chat-messages"><p><strong>Bob</strong>gg</p></div><div class="lobby-chat-compose"><input/><button>Send</button></div></div></div></section>'; });
  await page.waitForTimeout(200);
  // Force the look pile to render so its anchor is visible.
  const info = await page.evaluate(() => {
    const r = (s) => { const e = document.querySelector(s); if (!e) return null; const b = e.getBoundingClientRect(); return {x:Math.round(b.x),y:Math.round(b.y),w:Math.round(b.width),h:Math.round(b.height)}; };
    const piles = document.querySelector('.my-zone-board-shell .player-zone-piles');
    return {
      gy: r('.my-zone-board-shell [data-zone-pile="graveyard"]'),
      ex: r('.my-zone-board-shell [data-zone-pile="exile"]'),
      chat: r('.my-zone-board-shell .exile-chat-dock'),
      look: r('.my-zone-board-shell .player-look-pile'),
      piles: r('.my-zone-board-shell .player-zone-piles'),
      lookTop: piles && getComputedStyle(piles).getPropertyValue('--look-area-top'),
      field: r('.my-zone-board-shell .has-zone-piles'),
      board: r('.my-zone-board-shell'),
    };
  });
  console.log(JSON.stringify(info));
  await page.screenshot({path: out});
} finally { await browser.close(); await vite.close(); }
