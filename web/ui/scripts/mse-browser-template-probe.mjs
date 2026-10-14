import { createServer } from 'node:http';
import { readFile,writeFile,mkdir } from 'node:fs/promises';
import { resolve,dirname,basename } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { chromium } from 'playwright';
import upstream from '../src/lib/mse/upstream.mjs';
import {mseResourceFontPaths} from '../src/lib/mse/font-resources.mjs';
const args=process.argv.slice(2),fetchAssets=args.includes('--fetch-assets'),layers=args.includes('--layers'),useSession=args.includes('--session');
const fontDpi=Number(args.find(arg=>arg.startsWith('--font-dpi='))?.slice(11)??96);
if(![72,96].includes(fontDpi))throw new Error('Font DPI must be 72 (native macOS) or 96 (native Windows/Linux)');
const cardPath=args.find(arg=>arg.startsWith('--card='))?.slice(7);
const engineCardPath=args.find(arg=>arg.startsWith('--engine-card='))?.slice(14);
if(cardPath&&engineCardPath)throw new Error('Use either --card or --engine-card');
const snapshot=engineCardPath?JSON.parse(await readFile(resolve(engineCardPath),'utf8')):undefined;
const faceIndex=Number(args.find(arg=>arg.startsWith('--face-index='))?.slice(13)??0);
if(!Number.isInteger(faceIndex)||faceIndex<0)throw new Error('Invalid face index');
const ordered=args.includes('--ordered-fields');
const cardOptions=cardPath?JSON.parse(await readFile(resolve(cardPath),'utf8')):{};
const artPath=args.find(arg=>arg.startsWith('--art='))?.slice(6);
const artBytes=artPath?await readFile(resolve(artPath)):null;
if(artPath)cardOptions.card={...cardOptions.card,image:basename(artPath)};
const [directory,style='magic-m15-altered.mse-style',output='reports/mse/browser-template.json']=args.filter(arg=>!arg.startsWith('--'));
if(!directory || args.some(arg=>arg.startsWith('--')&&!['--fetch-assets','--layers','--session','--ordered-fields'].includes(arg)&&!arg.startsWith('--card=')&&!arg.startsWith('--engine-card=')&&!arg.startsWith('--art=')&&!arg.startsWith('--font-dpi=')&&!arg.startsWith('--face-index=')))throw new Error('Usage: node scripts/mse-browser-template-probe.mjs CORPUS_DIRECTORY [STYLE] [REPORT.json] [--fetch-assets] [--layers] [--session] [--card=CONTEXT.json | --engine-card=SNAPSHOT.json] [--art=IMAGE] [--font-dpi=72|96] [--face-index=N] [--ordered-fields]');
const root=resolve(directory),library=resolve(dirname(fileURLToPath(import.meta.url)),'../src/lib/mse');
const tree=JSON.parse(await readFile(resolve(root,'upstream-tree.json'),'utf8'));
if(tree.sha!==upstream.revision||tree.truncated)throw new Error('Expected a complete pinned corpus index');
const entries=new Map(tree.tree.filter(e=>e.type==='blob').map(e=>[e.path,e]));
const files=[...entries.keys()].filter(p=>p.startsWith('data/')).map(p=>p.slice(5));
const fontPaths=mseResourceFontPaths(entries.keys());
const fetchedAssets=[],pendingAssets=new Map();
async function readAsset(path,entry){
  if(!pendingAssets.has(path))pendingAssets.set(path,(async()=>{
    const local=resolve(root,path);if(!local.startsWith(root+'/'))throw new Error('Invalid corpus path');
    let bytes;
    try{bytes=await readFile(local);}catch(error){
      if(error.code!=='ENOENT'||!fetchAssets)throw error;
      const response=await fetch(`https://raw.githubusercontent.com/${upstream.repository}/${upstream.revision}/${path.split('/').map(encodeURIComponent).join('/')}`,{signal:AbortSignal.timeout(45000)});
      if(!response.ok)throw new Error(`Upstream asset HTTP ${response.status}: ${path}`);
      bytes=Buffer.from(await response.arrayBuffer());
      if(createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex')!==entry.sha)throw new Error('Downloaded asset hash mismatch');
      await mkdir(dirname(local),{recursive:true});await writeFile(local,bytes);fetchedAssets.push(path);
    }
    if(createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex')!==entry.sha)throw new Error('Corpus hash mismatch');
    return bytes;
  })().catch(error=>{pendingAssets.delete(path);throw error;}));
  return pendingAssets.get(path);
}
const server=createServer(async(req,res)=>{
  try{
    if(req.url==='/upstream-tree.json'){res.setHeader('Content-Type','application/json');res.end(JSON.stringify(tree));return;}
    if(req.url==='/__artwork'&&artBytes){res.end(artBytes);return;}
    if(req.url==='/'){res.setHeader('Content-Type','text/html');res.end('<!doctype html><title>MSE template layout probe</title>');return;}
    if(/^\/[a-z-]+\.mjs$/.test(req.url)){res.setHeader('Content-Type','text/javascript');res.end(await readFile(resolve(library,basename(req.url))));return;}
    const path=decodeURIComponent(req.url.slice(1));const entry=entries.get(path);
    if(!entry){res.writeHead(404);res.end('Not in pinned corpus');return;}
    const bytes=await readAsset(path,entry);
    res.end(bytes);
  }catch(error){res.writeHead(500);res.end(error.message);}
});
await new Promise(done=>server.listen(0,'127.0.0.1',done));
let browser;
try{
  browser=await chromium.launch({headless:true});const page=await browser.newPage();
  await page.goto(`http://127.0.0.1:${server.address().port}`);
  const result=await page.evaluate(async({files,fontPaths,style,layers,cardOptions,artName,useSession,fontDpi,snapshot,faceIndex,ordered})=>{
    const {MseBrowserFonts}=await import('/browser-fonts.mjs');
    const {createFetchMseStore}=await import('/packages.mjs');
    const {MseImageRenderer}=await import('/image-renderer.mjs');
    const {createMseImageLoader}=await import('/browser-images.mjs');
    const {createBrowserSymbolRenderer}=await import('/browser-symbols.mjs');
    const {MseCardRenderer}=await import('/card-renderer.mjs');
    const fonts=new MseBrowserFonts({allowSystemFonts:useSession,pixelsPerPoint:fontDpi/72}),fontFailures=[];let phase='fonts',field=null,session;
    const createCanvas=(w,h)=>{const c=document.createElement('canvas');c.width=w;c.height=h;return c;};
    const artworkLoader=artName?createMseImageLoader({assetUrl:()=>'/__artwork'}):null;
    const loadCardImage=artworkLoader?async value=>{if(value!==artName)throw new Error(`No supplied artwork for ${value}`);return artworkLoader({key:artName});}:undefined;
    try{
      if(useSession){
        const {createBrowserMseSession}=await import('/browser-session.mjs');
        session=await createBrowserMseSession({corpusUrl:new URL('/',location.href),createFonts:()=>fonts,createCanvas,loadCardImage});
        fontFailures.push(...session.fontFailures);
      }else for(const path of fontPaths){try{
        const response=await fetch('/'+path.split('/').map(encodeURIComponent).join('/'));if(!response.ok)throw new Error(await response.text());
        await fonts.registerFile(await response.arrayBuffer());
      }catch(error){fontFailures.push({path,message:error.message});}}
      const store=session?.store||createFetchMseStore(new URL('/data/',location.href),fetch,{files});
      const renderer=session?.renderer.images||new MseImageRenderer({loadAsset:createMseImageLoader(store),renderSymbol:createBrowserSymbolRenderer({store})});
      const service=session?.renderer||new MseCardRenderer({store,fonts,imageRenderer:renderer,loadCardImage,createCanvas,fieldUpdateMode:ordered?'ordered':cardOptions.fieldUpdateMode||'lazy'});
      phase='initialize';const context=await service.prepareContext({style,context:cardOptions,snapshot,
        adapter:{faceIndex:snapshot&&['split','adventure'].includes(snapshot.layout)?null:faceIndex,nativeCard:cardOptions.card||{}}});
      const original=context.layoutTaggedText.bind(context);
      context.layoutTaggedText=async(name,...args)=>{field=name;return original(name,...args);};
      const originalChoice=context.measureChoiceImage.bind(context);
      context.measureChoiceImage=async(name,...args)=>{field=name;return originalChoice(name,...args);};
      phase='draw';
      const stages=[];
      const {canvas,context:renderedContext,...drawn}=await service.renderContext(context,{
        onFieldDrawn:layers?({group,name,surface})=>{stages.push({name:`stage.${group}.${name}`,png:surface.toDataURL().split(',')[1]});}:undefined,
      });
      const measured=drawn.preparation;
      phase='layout';field=null;const layout=context.layout({visibleOnly:true});
      service.dispose();
      canvas.id='rendered-card';document.body.append(canvas);
      const diagnostics=[...stages,...context.diagnostics];
      if(layers)for(const f of layout.fields){
        if(!context.fieldVisible(f.name,f.group))continue;
        const raster=context.textMeasurements.get(`${f.group}.${f.name}`)?.result?.image;
        let png;
        if(raster){const c=document.createElement('canvas');c.width=raster.width;c.height=raster.height;
          const dc=c.getContext('2d'),p=dc.createImageData(c.width,c.height);p.data.set(raster.data);dc.putImageData(p,0,0);png=c.toDataURL().split(',')[1];}
        const textLayout=context.textMeasurements.get(`${f.group}.${f.name}`)?.result;
        diagnostics.push({name:`${f.group}.${f.name}`,properties:f.properties,png,
          textLayout:textLayout?.measurement?{scale:textLayout.scale,stretch:textLayout.stretch,lines:textLayout.lines}:undefined});
      }
      const measuredText=Object.fromEntries([...context.textMeasurements].filter(([,v])=>v.result?.measurement).map(([key,v])=>[key,v.result.measurement.runs.filter(r=>!r.soft).map(r=>r.text).join('')]));
      const measuredLayouts=Object.fromEntries([...context.textMeasurements].filter(([,v])=>v.result?.measurement).map(([key,v])=>[key,{scale:v.result.scale,stretch:v.result.stretch,alignment:context.property(key.slice(key.indexOf('.')+1),'alignment',key.slice(0,key.indexOf('.'))),geometry:Object.fromEntries(['left','top','width','height','content_width','content_height'].map(property=>[property,context.property(key.slice(key.indexOf('.')+1),property,key.slice(0,key.indexOf('.')))])),lines:v.result.lines.map(line=>[line.top,line.height,line.width])}]));
      return {success:true,phase,drawn,fontFailures,measuredLayouts,passes:measured.passes,textFields:measured.fields,dimensions:[layout.width,layout.height],fields:layout.fields.length,measuredText,diagnostics};
    }catch(error){return {success:false,phase,field:error.field||field,fontFailures,error:{code:error.code||'host',message:error.message,layoutChanges:error.layoutChanges}};}
    finally{if(session)session.dispose();else fonts.dispose();}
  },{files,fontPaths,style,layers,cardOptions,useSession,fontDpi,snapshot,faceIndex,ordered,artName:artPath?basename(artPath):null});
  for(const d of result.diagnostics||[]){if(!d.png)continue;
    d.imagePath=resolve(output.replace(/\.json$/i,'')+'.layers',d.name+'.png');
    await mkdir(dirname(d.imagePath),{recursive:true});await writeFile(d.imagePath,Buffer.from(d.png,'base64'));delete d.png;
  }
  let imagePath;
  if(result.success){
    imagePath=resolve(output.replace(/\.json$/i,'')+'.png');
    await mkdir(dirname(imagePath),{recursive:true});
    const data=await page.locator('#rendered-card').evaluate(canvas=>canvas.toDataURL('image/png').split(',')[1]);
    await writeFile(imagePath,Buffer.from(data,'base64'));
  }
  const report={upstream,style,engineCardPath:engineCardPath?resolve(engineCardPath):undefined,cardPath:cardPath?resolve(cardPath):undefined,artPath:artPath?resolve(artPath):undefined,generatedAt:new Date().toISOString(),fetchedAssets,fontSources:{policy:'native-resource-directories',files:fontPaths.length,allowSystemFonts:useSession},browserSession:useSession,fontDpi,faceIndex,scope:'Actual Chromium font metrics, template layout, and card drawing; no native visual parity comparison.',renderingVerified:false,imagePath,...result};
  await mkdir(dirname(resolve(output)),{recursive:true});await writeFile(output,JSON.stringify(report,null,2)+'\n');
  console.log(JSON.stringify(report,null,2));if(!result.success)process.exitCode=1;
}finally{await browser?.close();await new Promise(done=>server.close(done));}
