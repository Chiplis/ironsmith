import {chromium} from 'playwright';
import {createServer} from 'vite';
import fs from 'node:fs/promises';
process.chdir(new URL('../..',import.meta.url).pathname);
const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();const browser=await chromium.launch();
try{const page=await browser.newPage();await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
const result=await page.evaluate(async printing=>{
const {sampleCardFramePixels}=await import('/src/lib/card-frame-colors.js');const {cardTypography}=await import('/src/lib/card-typography.js');
const scan=async url=>{const im=new Image();im.src=url;await im.decode();const c=document.createElement('canvas');c.width=im.width;c.height=im.height;const ctx=c.getContext('2d');ctx.drawImage(im,0,0);return ctx.getImageData(0,0,c.width,c.height);};
const typography=cardTypography(printing);await Promise.all(['title','type','rules'].map(n=>document.fonts.load(`400 40px ${typography[n]}`)));
const fullScan=await scan('/test-results/ink/arnjlot.jpg');const style=await sampleCardFramePixels({fullScan,artScan:await scan('/test-results/ink/arnjlot-art.jpg'),printing,typography,icons:await (await import('/src/lib/card-mana-match.js')).manaTemplates(printing.mana_cost)});
document.body.innerHTML='';const original=new Image();original.src='/test-results/ink/arnjlot.jpg';document.body.append(original);const masked=new Image();masked.src=style['--source-frame-image']?.slice(5,-2);document.body.append(masked);await masked.decode().catch(()=>{});
const {hasOutlinedLightText}=await import('/src/lib/card-frame-font-mask.js'); const c=document.createElement('canvas');c.width=fullScan.width;c.height=fullScan.height;const ctx=c.getContext('2d');ctx.putImageData(fullScan,0,0);for(const [name,box] of Object.entries(JSON.parse(style['--printed-layout']))){if(name==='art')continue;const s=ctx.getImageData(box.x+6,box.y+6,box.width-12,box.height-12);style['debug-'+name]=[hasOutlinedLightText(s),globalThis.__inkLetters];} return style;},JSON.parse(await fs.readFile('/tmp/arnjlot-printing.json')));
await fs.writeFile('test-results/ink/style.json',JSON.stringify(result,null,2));console.log(Object.fromEntries(Object.entries(result).filter(([k])=>/ink|status|reason|text-bounds|layout$|debug/.test(k))));await page.screenshot({path:'test-results/ink/mask.png',fullPage:true});
}finally{await browser.close();await vite.close();}
