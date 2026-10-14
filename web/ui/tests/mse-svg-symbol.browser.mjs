// SVG set icons render through MSE symbol regions and rarity variations.
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
import {createServer} from 'vite';
import {chromium} from 'playwright';

const root=fileURLToPath(new URL('../',import.meta.url));
const icon='<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 10"><rect x="2" y="2" width="16" height="6" fill="#000"/></svg>';
const entry=`import {createBrowserSymbolRenderer,isSvgSymbolUrl} from '/src/lib/mse/browser-symbols.mjs';
const requests=[];
const fetcher=async url=>{requests.push(url);return new Response(${JSON.stringify(icon)},{headers:{'content-type':'image/svg+xml'}});};
const render=createBrowserSymbolRenderer({fetcher});
const variation={type:'solid',borderRadius:0.1,fill_color:[200,150,0,255],border_color:[0,0,0,255]};
window.run=async()=>{
 const url='https://svgs.example/sets/abc.svg?1';
 const [a,b]=await Promise.all([render({local:true,filename:url,variation},{width:20,height:10}),render({local:true,filename:url,variation},{width:20,height:10})]);
 const pixel=(x,y)=>Array.from(a.data.slice((y*a.width+x)*4,(y*a.width+x)*4+4));
 return {width:a.width,height:a.height,center:pixel(a.width>>1,a.height>>1),corner:pixel(0,0),
  edge:pixel(a.width>>1,Math.round(a.height*0.2)),requests:requests.length,same:a.width===b.width,
  urls:[isSvgSymbolUrl(url),isSvgSymbolUrl('symbol.mse-symbol'),isSvgSymbolUrl('http://x/a.svg')]};
};`;
const server=await createServer({root,configFile:false,logLevel:'error',cacheDir:`node_modules/.vite-mse-svg-symbol-${process.pid}`,
  resolve:{alias:{'@':fileURLToPath(new URL('../src',import.meta.url))}},optimizeDeps:{entries:[]},
  plugins:[{name:'svg-symbol',resolveId(id){if(id==='/svg-symbol.js')return id;},load(id){if(id==='/svg-symbol.js')return entry;},
    configureServer(server){server.middlewares.use((req,res,next)=>{if(req.url!=='/probe')return next();res.setHeader('Content-Type','text/html');res.end('<script type="module" src="/svg-symbol.js"></script>');});}}],
  server:{host:'127.0.0.1',port:0}});
let browser;
try{
  await server.listen();browser=await chromium.launch();const page=await browser.newPage();
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/probe`);await page.waitForFunction(()=>window.run);
  const result=await page.evaluate(()=>window.run());
  assert.equal(result.width/result.height,2,'the viewBox aspect is kept');
  assert.deepEqual(result.center,[200,150,0,255],'the icon inside takes the fill color');
  assert.deepEqual(result.corner,[0,0,0,0],'outside the icon is transparent');
  assert.deepEqual(result.edge,[0,0,0,255],'the dilated edge takes the border color');
  assert.equal(result.requests,1,'one download per icon');
  assert.deepEqual(result.urls,[true,false,false]);
  console.log(JSON.stringify({status:'passed',checks:['svg aspect','fill','outside','border','download coalescing','url detection']}));
}finally{await browser?.close();await server.close();}
