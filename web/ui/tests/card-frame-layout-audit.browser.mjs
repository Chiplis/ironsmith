// Run with CARD_FRAME_FIXTURES=/path/to/cache. Use CARD_FRAME_CASE_FILTER to
// select families/slugs; CARD_FRAME_AUDIT_OUTPUT holds reports and contact sheets.
// CARD_FRAME_LOCALE (e.g. es) previews translated text over the registered scans.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {join} from 'node:path';
import {chromium} from 'playwright';
import {createServer} from 'vite';

const root=fileURLToPath(new URL('../',import.meta.url));
const base=process.env.CARD_FRAME_FIXTURES;
assert.ok(base,'Set CARD_FRAME_FIXTURES; populate it with scripts/cache-card-frame-layouts.py.');
const output=process.env.CARD_FRAME_AUDIT_OUTPUT || join(base,'audit');
await mkdir(output,{recursive:true});
const manifest=JSON.parse(await readFile(new URL('./card-frame-layout-cases.json',import.meta.url),'utf8'));
const regressions=JSON.parse(await readFile(new URL('./card-frame-regression-cases.json',import.meta.url),'utf8'));
for(const regression of regressions)if(!manifest.some(c=>c.slug===regression.slug))manifest.push(regression);
const cases=manifest.filter(c=>!process.env.CARD_FRAME_CASE_FILTER || new RegExp(process.env.CARD_FRAME_CASE_FILTER).test(c.slug+' '+c.families.join(' ')));
const cached=await Promise.all(manifest.map(async c=>({case:c,printing:JSON.parse(await readFile(join(base,c.slug+'.json'),'utf8'))})));
const vite=await createServer({root,server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();
const browser=await chromium.launch();const results=[];
try {
  for(let start=0;start<cases.length;start+=4) {
    const batch=cases.slice(start,start+4);
    const prints=await Promise.all(batch.map(c=>readFile(join(base,c.slug+'.json'),'utf8').then(JSON.parse)));
    const faces=prints.map((p,i)=>batch[i].face!=null?p.card_faces[batch[i].face]:p);
    const cards=faces.map((face,i)=>({id:start+i+1,name:face.name,type_line:face.type_line,mana_cost:face.mana_cost,oracle_text:face.oracle_text,power:face.power,toughness:face.toughness,sourceImageUrl:batch[i].source}));
    const page=await browser.newPage({viewport:{width:1750,height:1400},reducedMotion:'reduce'});
    const errors=[];page.on('pageerror',error=>errors.push(error.message));
    await page.addInitScript(({cards,locale})=>{window.__comparisonCards=cards;window.__comparisonShowOriginals=true;if(locale)localStorage.setItem('ironsmith.locale',locale);},{cards,locale:process.env.CARD_FRAME_LOCALE||''});
    await page.route('https://cards.scryfall.io/**',async route=>{
      const url=route.request().url();const match=cached.find(({case:c})=>url.includes(c.id)&&url.includes(c.source.includes('/back/')?'/back/':'/front/'));
      if(!match)return route.abort();
      const variant=url.includes('/art_crop/')?'art_crop':'normal';
      try {await route.fulfill({contentType:'image/jpeg',headers:{'Access-Control-Allow-Origin':'*'},body:await readFile(join(base,match.case.slug+'-'+variant+'.jpg'))});}
      catch {await route.abort();}
    });
    await page.route('https://api.scryfall.com/**',route=>{
      const url=route.request().url();
      const match=cached.find(({printing:p})=>url.includes(p.id)||url.endsWith(`/cards/${p.set}/${p.collector_number}/${p.lang}`));
      const request=new URL(url);
      if(request.pathname==='/cards/named') {
        const name=request.searchParams.get('exact')||request.searchParams.get('fuzzy');
        return route.fulfill({json:cached.find(({printing:p})=>p.lang==='en'&&(p.name===name||p.card_faces?.some(f=>f.name===name)))?.printing||{}});
      }
      if(request.pathname==='/cards/search') {
        const query=request.searchParams.get('q')||'';
        return route.fulfill({json:{data:cached.filter(({printing:p})=>query.includes(p.oracle_id)&&query.includes(`lang:${p.lang}`)).map(c=>c.printing)}});
      }
      return route.fulfill({json:match?.printing||{}});
    });
    await page.route('https://svgs.scryfall.io/**',route=>route.abort());
    await page.route('**/cards/*.json',route=>route.fulfill({json:{}}));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
    await page.waitForFunction(n=>document.querySelectorAll('[data-render-ready="true"]').length===n,batch.length,{timeout:90000});
    if(process.env.CARD_FRAME_LOCALE)await page.evaluate(async({cards,locale})=>{
      const {loadTranslatedCardView}=await import('/src/i18n/cardTranslations.js');
      await Promise.all(cards.map(c=>loadTranslatedCardView(locale,{name:c.name,typeLine:c.type_line,rulesText:c.oracle_text})));
    },{cards,locale:process.env.CARD_FRAME_LOCALE});
    await page.evaluate(async()=>{
      await document.fonts.ready;
      await Promise.allSettled([...document.images].map(img=>img.decode()));
      // Registered replacements mount after their mask patches arrive and the
      // fitter runs a frame later: wait for the field set and text rects to
      // hold still, not merely for the first frame with no rule lines.
      await new Promise(resolve=>{let last='',stable=0,frames=0;const check=()=>{const next=JSON.stringify([...document.querySelectorAll('.interactive-card-frame__rule-line, .registered-card-frame__field')].map(el=>{const r=el.getBoundingClientRect();return [el.dataset.replaced,r.x,r.y,r.width,r.height,getComputedStyle(el).fontSize,el.style.cssText];}));stable=next===last?stable+1:0;last=next;if(stable>=20 || ++frames>=600)resolve();else requestAnimationFrame(check);};requestAnimationFrame(check);});
    });
    const actual=await page.locator('.interactive-card-frame-stage').evaluateAll(nodes=>nodes.map(n=>{
      const get=k=>n.style.getPropertyValue(k),parse=k=>JSON.parse(get(k)||'null');
      const frame=n.querySelector('.interactive-card-frame');
      const rules=n.querySelector('.interactive-card-frame__rules');
      const rulesRect=rules?.getBoundingClientRect();
      return {mode:n.dataset.frameMode,reason:get('--source-frame-fallback-reason')||null,boxes:parse('--printed-layout'),
        mana:parse('--printed-mana-symbols')?.symbols.map(({image,...s})=>s),
        titleBounds:parse('--printed-title-text-bounds'),typeBounds:parse('--printed-type-text-bounds'),statsBounds:parse('--printed-stats-text-bounds'),
        title:frame?.querySelector('.interactive-card-frame__title')?.textContent,
        rules:frame?.querySelector('.interactive-card-frame__rules')?.textContent,
        image:n.querySelector('.original-card-fallback > img')?.src,
        typography:rules?{printed:get('--printed-rules-font-size'),printedFlavor:get('--printed-flavor-font-size'),first:parse('--printed-rules-first-line'),font:getComputedStyle(rules).fontSize,rect:rules.getBoundingClientRect().toJSON(),textRects:[...rules.querySelectorAll('.interactive-card-frame__rule-line')].map(n=>{const r=document.createRange();r.selectNodeContents(n);return r.getBoundingClientRect().toJSON();}),fitted:rules.style.cssText}:null,
        ruleMetrics:rules?{height:rules.clientHeight,scroll:rules.scrollHeight,rows:[...rules.querySelectorAll('.interactive-card-frame__rule')].map(row=>({top:row.getBoundingClientRect().top-rulesRect.top,height:row.getBoundingClientRect().height,text:row.textContent}))}:null,
        sourceMask:!!get('--source-frame-image'),assembled:!!frame&&!get('--source-frame-image'),
        textOverflow:[...n.querySelectorAll('.interactive-card-frame__title,.interactive-card-frame__type')].some(el=>el.scrollWidth>el.clientWidth+2),
        fonts:[...document.fonts].filter(f=>f.status!=='unloaded').map(f=>`${f.family} ${f.weight} ${f.style}:${f.status}`),
        registered:[...n.querySelectorAll('.registered-card-frame__field[data-replaced="true"]')].map(f=>{const box=f.querySelector('.interactive-card-frame__rules');const range=document.createRange();range.selectNodeContents(box);const r=range.getBoundingClientRect();return {kind:f.dataset.fieldKind,text:f.dataset.liveText,size:f.style.getPropertyValue('--registered-field-font-size'),font:getComputedStyle(box).fontSize,fit:box.style.getPropertyValue('--card-rules-fit-scale'),color:getComputedStyle(box.querySelector('.interactive-card-frame__rule-line')||box).color,box:[box.clientWidth,box.clientHeight],text_box:[Math.round(r.width),Math.round(r.height)]};}),
      };
    }));
    if(process.env.CARD_FRAME_DIAGNOSTICS==='1')for(let i=0;i<batch.length;i++) {
      actual[i].diagnostics=await page.evaluate(async url=>{
        const api=await import('/src/lib/card-frame-colors.js');
        const scan=async(src,width)=>{const img=new Image();img.crossOrigin='anonymous';img.src=src;await img.decode();const canvas=document.createElement('canvas');canvas.width=width;canvas.height=Math.round(img.height*width/img.width);const ctx=canvas.getContext('2d');ctx.drawImage(img,0,0,canvas.width,canvas.height);return ctx.getImageData(0,0,canvas.width,canvas.height);};
        const full=await scan(url,488),art=await scan(url.replace('/normal/','/art_crop/'),160);
        return {art:api.matchArtBounds(full,art),title:api.classifyTitlePanel(full),type:api.classifyTypePanel(full),stats:api.detectPrintedStats(full),boxes:Object.fromEntries(['title','type','rules'].map(section=>[section,api.detectPanelBounds(full,section)])),geometry:api.measureFrameGeometry(full,art)};
      },batch[i].source);
    }
    for(let i=0;i<batch.length;i++) {
      const result={...batch[i],...actual[i],errors};results.push(result);
      assert.ok(!result.assembled,`${result.slug}: assembled panel regression`);
      if(batch[i].face!=null)for(const field of result.registered) {
        if(['name','type'].includes(field.kind))assert.ok(!field.text.includes('//'),`${result.slug}: combined face text in ${field.kind}`);
      }
      if(result.mode==='original')assert.equal(result.image,batch[i].source,`${result.slug}: fallback changed printing/face`);
      console.log(`${start+i+1}/${cases.length} ${result.slug}: ${result.mode} ${result.reason||''}`);
    }
    await page.screenshot({path:join(output,`sheet-${String(start/4+1).padStart(2,'0')}.png`),fullPage:true,animations:'disabled'});
    await writeFile(join(output,'results.json'),JSON.stringify(results,null,2));
    await page.close();
  }
} finally {await browser.close();await vite.close();}
const summary={cases:results.length,masked:results.filter(c=>c.mode==='masked').length,registered:results.filter(c=>c.mode==='registered').length,original:results.filter(c=>c.mode==='original').length,
  ruleOverflow:results.filter(c=>c.ruleMetrics&&(c.ruleMetrics.scroll>c.ruleMetrics.height+2||c.ruleMetrics.rows.some(r=>r.top+r.height>c.ruleMetrics.height+2))).map(c=>c.slug),
  errors:results.filter(c=>c.errors.length).map(c=>c.slug),overflow:results.filter(c=>c.textOverflow).map(c=>c.slug)};
await writeFile(join(output,'summary.json'),JSON.stringify(summary,null,2));
console.log(JSON.stringify(summary,null,2));
assert.deepEqual(summary.errors,[],'browser errors');
if(process.env.CARD_FRAME_ASSERT_EXPECTATIONS==='1')for(const result of results)if(result.expectedMode)assert.equal(result.mode,result.expectedMode,result.slug);
const escapeHtml=value=>String(value).replace(/[&<>"']/g,char=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[char]));
await writeFile(join(output,'index.html'),`<!doctype html><meta charset="utf-8"><title>Card frame layout audit</title><style>body{background:#101318;color:#eee;font:16px system-ui;margin:24px}img{width:100%;height:auto}section{margin:32px 0}pre{white-space:pre-wrap}</style><h1>Card frame layout audit</h1><p>Original scan on the left; current interactive preview on the right. Masked mode is not a visual-quality pass.</p><pre>${escapeHtml(JSON.stringify(summary,null,2))}</pre>${Array.from({length:Math.ceil(results.length/4)},(_,i)=>`<section><h2>${escapeHtml(results.slice(i*4,i*4+4).map(c=>`${c.name}: ${c.mode}${c.reason?' ('+c.reason+')':''}`).join(' / '))}</h2><img loading="lazy" src="sheet-${String(i+1).padStart(2,'0')}.png"></section>`).join('')}`);
