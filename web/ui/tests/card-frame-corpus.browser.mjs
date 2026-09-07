// Optional visual corpus check. Supply cached Scryfall JSON and normal/art_crop
// JPEGs named by slug in CARD_FRAME_FIXTURES; no live network is needed.
import assert from 'node:assert/strict';
import cases from './card-frame-printings.js';
import {fileURLToPath} from 'node:url';
import {createRequire} from 'node:module';
import {readFileSync, writeFileSync} from 'node:fs';
const require=createRequire(import.meta.url);
const {chromium}=require('playwright');const {createServer}=await import('vite');
const root=fileURLToPath(new URL('../',import.meta.url));
if (!process.env.CARD_FRAME_FIXTURES) throw new Error('Set CARD_FRAME_FIXTURES to the cached printing corpus directory.');
const base=process.env.CARD_FRAME_FIXTURES.replace(/\/$/,'')+'/';
const slugs=cases.map(c=>c.slug);
const prints=slugs.map(slug=>JSON.parse(readFileSync(base+slug+'.json')));
prints.forEach((printing,i)=>assert.equal(printing.id,cases[i].id,`Wrong printing for ${cases[i].name}`));
const cards=prints.map((c,i)=>({id:i+1,name:c.name,type_line:c.type_line,mana_cost:c.mana_cost,oracle_text:c.oracle_text,power:c.power,toughness:c.toughness}));
const vite=await createServer({root,server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();const browser=await chromium.launch();
try{
const page=await browser.newPage({viewport:{width:1824,height:1916},deviceScaleFactor:2});
page.setDefaultTimeout(180000);
await page.addInitScript(cards=>window.__comparisonCards=cards,cards);
await page.route('https://cards.scryfall.io/**',r=>{const i=prints.findIndex(p=>r.request().url().includes(p.id));return i<0?r.abort():r.fulfill({contentType:'image/jpeg',headers:{'Access-Control-Allow-Origin':'*'},body:readFileSync(base+slugs[i]+'-'+(r.request().url().includes('/art_crop/')?'art_crop':'normal')+'.jpg')});});
await page.route('https://api.scryfall.com/**',r=>{
  const setCode=r.request().url().match(/\/sets\/([^?]+)/)?.[1];
  if(setCode)try{return r.fulfill({json:JSON.parse(readFileSync(base+setCode+'-set.json'))});}catch{return r.fulfill({json:{}});}
  const i=prints.findIndex(p=>r.request().url().includes(p.id));return r.fulfill({json:i>=0?prints[i]:{}});
});
await page.route('https://svgs.scryfall.io/**',r=>{
  const code=r.request().url().match(/\/sets\/([^.?]+)/)?.[1];
  try{return r.fulfill({contentType:'image/svg+xml',headers:{'Access-Control-Allow-Origin':'*'},body:readFileSync(base+code+'-symbol.svg')});}catch{return r.abort();}
});
for(let i=0;i<slugs.length;i++) await page.route('**/cards/'+slugs[i]+'.json',r=>r.fulfill({json:{scryfall:prints[i]}}));
await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`, {waitUntil:"commit",timeout:180000});
await page.waitForFunction(n=>document.querySelectorAll('[data-printing-ready="true"][data-card-colors="sampled"]').length===n,cases.length);
await page.waitForFunction(n=>document.querySelectorAll('[aria-label="Flavor text"]').length===n, prints.filter(p=>p.flavor_text).length);
await page.waitForFunction(n=>document.querySelectorAll('[data-render-ready="true"]').length===n,cases.length);
await page.waitForFunction(()=>document.querySelector('[data-comparison-card="0"] [data-card-era="retro"]') && !document.querySelector('[data-comparison-card="7"] [data-title-panel]'));
await page.evaluate(async()=>{await document.fonts.ready;await Promise.all([...document.querySelectorAll('.interactive-card-frame__art img')].map(i=>i.decode()));});
const fontMaskCheck=await page.evaluate(async()=>{
  const {fontGuidedPanel}=await import('/src/lib/card-frame-font-mask.js');
  const canvas=document.createElement('canvas');canvas.width=320;canvas.height=80;
  const ctx=canvas.getContext('2d');ctx.fillStyle='#e0e0dc';ctx.fillRect(0,0,320,80);
  ctx.fillStyle='#151515';ctx.font='400 20px "MPlantin"';ctx.fillText('When this creature dies.',13,33);
  ctx.fillRect(8,65,304,2);
  const scan=ctx.getImageData(0,0,320,80);
  const result=fontGuidedPanel(scan,{family:'"MPlantin"',weight:400,text:'When this creature dies.',section:'rules'});
  let ink=0,covered=0;
  for(let y=8;y<45;y++)for(let x=8;x<280;x++)if(scan.data[(y*320+x)*4]<80){ink++;if(result.mask[y*320+x])covered++;}
  let rim=0;for(let x=8;x<312;x++)rim+=result.mask[65*320+x];
  return {coverage:covered/ink,rim,matches:result.matches};
});
assert.ok(fontMaskCheck.coverage>.93,'font templates cover translated rendered glyphs '+JSON.stringify(fontMaskCheck));
assert.equal(fontMaskCheck.rim,0,'font mask preserves a non-text rule');
console.log(await page.locator('.interactive-card-frame-stage').evaluateAll(els=>els.map(el=>({kind:el.dataset.titlePanel,type:el.dataset.typePanel,confidence:el.style.getPropertyValue('--title-panel-confidence'),pt:el.style.getPropertyValue('--printed-pt-drop'),gaps:['title-art','art-type','type-rules'].map(n=>el.style.getPropertyValue('--printed-gap-'+n))}))));
const actual=await page.locator('.interactive-card-frame-stage').evaluateAll(els=>els.map(el=>({
  textBounds:Object.fromEntries(['title','type','stats'].map(s=>[s,JSON.parse(el.style.getPropertyValue(`--printed-${s}-text-bounds`)||'null')])),
  firstLine:JSON.parse(el.style.getPropertyValue('--printed-rules-first-line')||'null'),
  headerBaseline:Number(el.style.getPropertyValue('--printed-title-baseline')),manaMatch:JSON.parse(el.style.getPropertyValue('--printed-mana-symbols')||'null'),
  symbol:JSON.parse(el.style.getPropertyValue('--printed-set-symbol-bounds')||'null'),
  titlePanel:el.dataset.titlePanel || null,typePanel:el.dataset.typePanel || null,
  maskMethod:el.style.getPropertyValue('--source-frame-mask-method'),
  material:el.style.getPropertyValue('--frame-material-sampling'),
  artEnclosure:el.dataset.artEnclosure || null,
  titleBackground:getComputedStyle(el.querySelector('.interactive-card-frame__title-row')).backgroundImage,
  typeBackground:getComputedStyle(el.querySelector('.interactive-card-frame__type-row')).backgroundImage,
  innerBorder:el.dataset.innerFrameBorder || null,bevel:el.querySelectorAll('.interactive-card-frame__bevel').length,
  title:el.style.getPropertyValue('--whole-title-image'),type:el.style.getPropertyValue('--whole-type-image'),rules:el.style.getPropertyValue('--whole-rules-image'),
})));
actual.forEach((result,i)=>{
  if(cases[i].slug==='braids-arisen-nightmare') {
    assert.deepEqual(result.textBounds.type,{x:42,y:392,width:324,height:21},'Braids type text is measured inside its panel');
    assert.equal(result.firstLine?.line,'At the beginning of your end step,','Braids rules template matches the printed first line');
    assert.equal(result.firstLine.lineHeight,23,'Braids spacing comes from consecutive printed lines');
    assert.equal(result.manaMatch?.symbols.length,3,'Braids keeps all three mana symbols');
    assert.equal(result.maskMethod,'font-template','Braids retains the original printing');
  }
  if(cases[i].slug==='omniscience') {
    assert.ok(result.headerBaseline>=59&&result.headerBaseline<=63,'Omniscience printed title baseline');
    const m=result.manaMatch;
    assert.equal(m?.symbols.length,4,'Omniscience matches all four mana SVGs');
    assert.ok(m.symbols[0].x<359&&m.symbols.at(-1).x>422,'Omniscience SVGs register against source positions '+JSON.stringify(m));
  }
  if(cases[i].innerBorder)assert.ok(result.manaMatch,cases[i].name+' mana SVG registration');
  if(cases[i].innerBorder)assert.equal(result.maskMethod,'font-template',cases[i].name+' uses loaded font templates');
  assert.equal(result.material,cases[i].innerBorder?'regional':'',cases[i].name+' regional background or nonstandard fallback');
  assert.equal(result.artEnclosure,i<2?'detected':null,cases[i].name+' embedded art enclosure');
  if(result.titlePanel==='integrated') assert.equal(result.titleBackground,'none',cases[i].name+' shared title surface');
  if(result.typePanel==='integrated') assert.equal(result.typeBackground,'none',cases[i].name+' shared type surface');
  assert.equal(result.innerBorder,cases[i].innerBorder,cases[i].name+' inner rim classification');
  assert.equal(result.bevel,result.innerBorder?1:0,cases[i].name+' single bevel treatment');
  assert.equal(result.titlePanel,cases[i].titlePanel,cases[i].name+' title classification');
  assert.equal(result.typePanel,cases[i].typePanel,cases[i].name+' type classification');
  for(const section of ['title','type']) if(result[section+'Panel']==='panel') assert.match(result[section],/image\/svg\+xml/,cases[i].name+' '+section+' vector outline');
  if(result.titlePanel==='panel'||result.typePanel==='panel') assert.match(result.rules,/image\/svg\+xml/,cases[i].name+' rules outline');
});
const checkGeometry=async()=>{
const geometry=await page.locator('.interactive-card-frame-stage').evaluateAll(els=>els.map(el=>{
  const rect=selector=>el.querySelector(selector)?.getBoundingClientRect();
  const rules=rect('.interactive-card-frame__rules'),frame=el.querySelector('.interactive-card-frame').getBoundingClientRect();
  const titleBox=rect('.interactive-card-frame__title-row'),art=rect('.interactive-card-frame__art'),type=rect('.interactive-card-frame__type-row');
  const ptNode=el.querySelector('.interactive-card-frame__printed-stats'),pt=ptNode?.getBoundingClientRect(),flavor=rect('[aria-label="Flavor text"]');
  const statsStyle=ptNode?getComputedStyle(ptNode):null;
  const inner=el.querySelector('.interactive-card-frame__inner'),bevel=el.querySelector('.interactive-card-frame__bevel');
  const rim=bevel&&el.dataset.sourceFrame!=='true'?{shadow:getComputedStyle(inner).boxShadow,border:getComputedStyle(inner).borderTopWidth,z:getComputedStyle(bevel).zIndex,
    radius:parseFloat(getComputedStyle(bevel).borderTopLeftRadius),
    bounds: bevel.getBoundingClientRect().toJSON(), expected: JSON.parse(el.style.getPropertyValue('--frame-border-bounds') || 'null'),
    outerColor: getComputedStyle(el.querySelector('.interactive-card-frame')).backgroundColor,
    innerBackground: getComputedStyle(inner).backgroundImage,
    width:bevel.width,expectedWidth:bevel.getBoundingClientRect().width*Math.min(window.devicePixelRatio,2),
    cornerAlpha:bevel.getContext('2d').getImageData(0,0,1,1).data[3],
    centerAlpha:bevel.getContext('2d').getImageData(Math.floor(bevel.width/2),Math.floor(bevel.height/2),1,1).data[3],
    oldOutline:getComputedStyle(inner,'::after').borderImageSource}:null;
  const scale=frame.width/100;
  const panelText=['title','type'].map(kind=>{
    const node=el.querySelector('.interactive-card-frame__'+kind),range=document.createRange();range.selectNodeContents(node);
    return {kind,lines:new Set([...range.getClientRects()].map(r=>r.y)).size,width:range.getBoundingClientRect().width,available:node.getBoundingClientRect().width};
  });
  const symbolBounds=JSON.parse(el.style.getPropertyValue('--printed-set-symbol-bounds')||'null');
  const typeRange=document.createRange();typeRange.selectNodeContents(el.querySelector('.interactive-card-frame__type'));
  const typeTextRight=typeRange.getBoundingClientRect().right;
  const boxSizing = el.dataset.boxSizing;
  const source = JSON.parse(el.style.getPropertyValue('--printed-layout') || 'null');
  const sourceScale = frame.width / Number(el.style.getPropertyValue('--printed-scan-width'));
  const boxes = source ? ['title', 'type', 'rules', 'art'].map((kind, i) => ({kind,
    actual: {x:[titleBox,type,rules,art][i].x-frame.x, y:[titleBox,type,rules,art][i].y-frame.y,
      width:[titleBox,type,rules,art][i].width, height:[titleBox,type,rules,art][i].height},
    expected: Object.fromEntries(['x','y','width','height'].map(d=>[d,source[kind][d]*sourceScale])),
  })) : [];
  const rulesNode = el.querySelector('.interactive-card-frame__rules');
  const artImage = el.querySelector('.interactive-card-frame__art img');
  return {symbolBounds,typeTextRight,sourceFrame:el.dataset.sourceFrame==='true', sourceBackground:getComputedStyle(el.querySelector('.interactive-card-frame')).backgroundImage, frameX:frame.x, frameY:frame.y, frameWidth: frame.width, frameHeight: frame.height, artFit: artImage?getComputedStyle(artImage).objectFit:'contain', boxSizing, boxes, rulesFont: parseFloat(getComputedStyle(rulesNode).fontSize), rulesWidthOverflow: rulesNode.scrollWidth-rulesNode.clientWidth, rulesOverflow: rulesNode.scrollHeight-rulesNode.clientHeight, rim,panelText,outerPadding:getComputedStyle(el.querySelector('.interactive-card-frame')).padding,frameBottom:frame.bottom,treatment:el.querySelector('[data-pt-treatment]')?.dataset.ptTreatment,statsBorder:statsStyle?.borderTopWidth,statsBackground:statsStyle?.backgroundImage,fontSize:statsStyle?.fontSize,pt:pt?{x:pt.x,right:pt.right,y:pt.y,bottom:pt.bottom,center:pt.y+pt.height/2}:null,rulesRight:rules.right,rulesBottom:rules.bottom,flavorBottom:flavor?.bottom,
    expectedDrop:statsStyle ? -parseFloat(statsStyle.bottom) : 0,
    gaps:['title-art','art-type','type-rules'].map((name,i)=>({value:el.style.getPropertyValue('--printed-gap-'+name),actual:[art.top-titleBox.bottom,type.top-art.bottom,rules.top-type.bottom][i],scale}))};
}));
writeFileSync(base + `box-geometry-${Math.round(await page.locator('[data-comparison-card="0"]').evaluate(el=>el.getBoundingClientRect().width))}.json`, JSON.stringify(geometry, null, 2));
geometry.forEach((g,i)=>{
  if(g.boxSizing==='measured') {assert.ok(g.sourceFrame,cases[i].name+' uses original masked printing');assert.match(g.sourceBackground,/data:image\/png/,cases[i].name+' original frame image');}
  if(g.symbolBounds)assert.ok(g.typeTextRight<g.frameX+g.symbolBounds.x*g.frameWidth/488,cases[i].name+' type text stops before the set icon');
  assert.equal(g.artFit, 'contain', cases[i].name+' resizes the complete artwork');
  assert.ok(g.rulesFont >= g.gaps[0].scale * 2.5, cases[i].name+' rules remain readable: '+g.rulesFont);
  assert.ok(g.rulesWidthOverflow <= 1, cases[i].name+' rules fit horizontally');
  assert.equal(g.boxSizing === 'measured', ![7, 9].includes(i), cases[i].name+' measured conventional boxes or nonstandard fallback');
  if (g.boxSizing === 'measured') {
    for (const box of g.boxes) for (const d of ['x','y','width','height']) {
      assert.ok(Math.abs(box.actual[d]-box.expected[d])<1.5, cases[i].name+' measured '+box.kind+' '+d+' '+JSON.stringify(box));
    }
    const rules = g.boxes.find(b=>b.kind==='rules');
    assert.ok(rules.actual.x > g.gaps[0].scale*4, cases[i].name+' source textbox left inset');
    // Visible parchment edges in the cached source printings, independent
    // of the generated CSS: roughly 49..439/446 in a 488px scan.
    if (['yawgmoth-thran-physician', 'chrome-mox'].includes(cases[i].slug)) {
      const left = rules.actual.x/g.frameWidth;
      const right = (rules.actual.x+rules.actual.width)/g.frameWidth;
      assert.ok(left > .09 && left < .11, cases[i].name+' original left parchment margin');
      assert.ok(right > .89 && right < .92, cases[i].name+' original right parchment margin');
    }
    assert.ok(Math.abs(g.frameWidth/g.frameHeight - 488/680)<.01, cases[i].name+' uniform source scale');
    assert.ok(g.rulesOverflow <= 1, cases[i].name+' complete rules fit');
  }
  for(const line of g.panelText) {
    assert.equal(line.lines,1,cases[i].name+' '+line.kind+' stays on one line');
    assert.ok(line.width<=line.available+0.05,cases[i].name+' '+line.kind+' fits the panel');
  }
  assert.equal(g.outerPadding,'2px',cases[i].name+' keeps existing black border');
  if(g.rim) {
    assert.equal(g.rim.shadow,'none',cases[i].name+' no competing inset shadow');
    assert.equal(g.rim.border,'0px',cases[i].name+' no duplicate inner outline');
    assert.equal(g.rim.oldOutline,'none',cases[i].name+' no rectangular scan outline');
    assert.ok(Number(g.rim.z)>2,cases[i].name+' bevel above overlapping section effects');
    if (g.boxSizing === 'measured') {
      const scale=g.frameWidth/488;
      for (const d of ['width','height']) assert.ok(Math.abs(g.rim.bounds[d]-g.rim.expected[d]*scale)<1, cases[i].name+' inset bevel '+d);
      assert.ok(Math.abs(g.rim.bounds.x-g.frameX-g.rim.expected.x*scale)<1, cases[i].name+' border left margin');
      assert.ok(Math.abs(g.rim.bounds.y-g.frameY-g.rim.expected.y*scale)<1, cases[i].name+' border top margin');
      assert.equal(g.rim.innerBackground,'none',cases[i].name+' texture does not cover the physical border');
      if (i<2) {
        assert.ok(g.rim.expected.x>=20 && g.rim.expected.x<=28, cases[i].name+' original black border thickness');
        assert.ok(g.rim.radius<=2, cases[i].name+' nearly square retro interior');
      }
    }
    assert.ok(Math.abs(g.rim.width-g.rim.expectedWidth)<=1,cases[i].name+' bevel follows resizing');
    if (g.rim.radius>2) assert.equal(g.rim.cornerAlpha,0,cases[i].name+' rounded corner remains transparent');
    assert.equal(g.rim.centerAlpha,0,cases[i].name+' transparent bevel interior');
  }

  const creature=prints[i].power!=null && prints[i].toughness!=null;
  assert.equal(Boolean(g.pt),creature,cases[i].name+' detected P/T');
  if(g.pt) {
    assert.ok(parseFloat(g.fontSize)>0,cases[i].name+' visible P/T font');
    assert.equal(g.treatment,prints[i].frame==='1997'?'text':'panel',cases[i].name+' P/T enclosure');
    if(g.treatment==='text') {assert.equal(g.statsBorder,'0px');assert.equal(g.statsBackground,'none');}
    assert.ok(g.pt.bottom<=g.frameBottom-2,cases[i].name+' P/T stays inside the card');
    assert.ok(Math.abs(g.pt.center-g.rulesBottom-g.expectedDrop)<1.5,cases[i].name+' original P/T offset');
    if(g.flavorBottom) assert.ok(g.flavorBottom<=g.pt.y+1,cases[i].name+' flavor stays above P/T '+JSON.stringify(g));
  }
  for(const [index,gap] of g.gaps.entries()) {
    if (g.boxSizing === 'measured') continue;
    const floor=(index===0?cases[i].titlePanel:cases[i].typePanel)==='panel'?2:0;
    if(floor) assert.ok(gap.actual>=floor,cases[i].name+' at least 2px at '+['name/art','art/type','type/rules'][index]);
    if(gap.value) assert.ok(Math.abs(gap.actual-Math.max(floor,parseFloat(gap.value)*gap.scale))<1.5,cases[i].name+' measured section spacing');
  }
});
};
await checkGeometry();
await page.screenshot({path:base+'expanded-comparison.png'});
const cornerBox=await page.locator('[data-comparison-card="0"]').boundingBox();
await page.screenshot({path:base+'ornithopter-rim-corners.png',clip:{...cornerBox,height:115}});
for(let i=0;i<cases.length;i++) {
  const source=await page.locator(`[data-comparison-card="${i}"] .interactive-card-frame-stage`).evaluate(el=>el.style.getPropertyValue('--source-frame-image'));
  const base64=source.match(/base64,([^"\)]+)/)?.[1];
  if(base64)writeFileSync(base+slugs[i]+'-masked.png',Buffer.from(base64,'base64'));
}
for(let i=0;i<cases.length;i++) await page.locator(`[data-comparison-card="${i}"]`).screenshot({path:base+slugs[i]+'.png'});
await page.locator('[data-comparison-card]').evaluateAll(els=>els.forEach(el=>{el.style.width='240px';el.style.height='396px';}));
await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
await checkGeometry();
await page.locator('[data-comparison-card="0"]').screenshot({path:base+'ornithopter-240.png'});
for (const [width, height] of [[350.79, 490.08], [380, 531], [240, 335], [180, 252], [600,420]]) {
  await page.locator('[data-comparison-card]').evaluateAll((els, size)=>els.forEach(el=>{el.style.width=size[0]+'px';el.style.height=size[1]+'px';}), [width,height]);
  await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
  await checkGeometry();
  await page.locator('[data-comparison-card="1"]').screenshot({path:base+`yawgmoth-${width}x${height}.png`});
}
console.log('Rendered '+cases.length+' exact printings and checked geometry and visible rules at seven preview sizes; see expanded-comparison.png in the fixture directory.');
}finally{await browser.close();await vite.close();}
