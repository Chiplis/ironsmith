import {readFileSync,writeFileSync} from 'node:fs';
import {chromium} from 'playwright';
const output=process.env.HIGHLIGHT_OUTPUT;
if(!output) throw new Error('Set HIGHLIGHT_OUTPUT.');
const cards=[['yawgmoth-thran-physician','Yawgmoth, Thran Physician','First activated ability on the stack'],['myr-moonvessel','Myr Moonvessel','Death trigger on the stack']];
const styles=`*{box-sizing:border-box}body{margin:0;padding:28px;background:#0b0e12;color:#f4efe5;font-family:Arial,sans-serif}section{width:1020px;padding:24px;background:#11151a;margin:0 auto 28px;border:1px solid #30343a;border-radius:12px}h1{font-size:23px;font-weight:600;margin:0 0 7px}p{font-size:13px;color:#a7afb8;margin:0 0 22px}.pair{display:grid;grid-template-columns:1fr 1fr;gap:16px}.label{font-size:11px;font-weight:600;letter-spacing:2px;text-transform:uppercase;padding:10px 14px;background:#1b2026;border:1px solid #30343a;border-radius:6px;margin-bottom:10px}.after .label{color:#f3d18a;border-color:#6e5830;background:#242019}img{display:block;width:100%;height:auto}.detail{height:260px;overflow:hidden}.detail img{width:100%;transform:translateY(-516px)}.detail-section h1{font-size:20px}.detail-section{margin-bottom:20px}`;
function markup(inline=false,detail=false){return `<!doctype html><html><meta charset="UTF-8"><title>Ability highlight · Before and after</title><style>${styles}</style><body>${cards.map(([slug,name,caption])=>`<section data-card="${slug}" class="${detail?'detail-section':''}"><h1>${name}</h1><p>${caption}</p><div class="pair">${['before','after'].map(version=>{const file=`${slug}-${version}.png`;const src=inline?`data:image/png;base64,${readFileSync(`${output}/${file}`).toString('base64')}`:file;return `<div class="${version}"><div class="label">${version}</div><div class="${detail?'detail':''}"><img alt="${name} — ${version}" src="${src}"></div></div>`}).join('')}</div></section>`).join('')}</body></html>`}
writeFileSync(`${output}/comparison.html`,markup());
const browser=await chromium.launch();
try{
 const page=await browser.newPage({viewport:{width:1080,height:1200},deviceScaleFactor:1.5});
 await page.setContent(markup(true));
 await page.evaluate(()=>Promise.all([...document.images].map(img=>img.decode())));
 for(const [slug] of cards) await page.locator(`[data-card="${slug}"]`).screenshot({path:`${output}/${slug}-comparison.png`});
 await page.setContent(markup(true,true));
 await page.evaluate(()=>Promise.all([...document.images].map(img=>img.decode())));
 await page.setViewportSize({width:1080,height:1});
 await page.screenshot({path:`${output}/highlight-details-comparison.png`,fullPage:true});
 console.log(`${output}/comparison.html`);
}finally{await browser.close()}
