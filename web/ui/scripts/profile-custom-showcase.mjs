import fs from 'node:fs';
import {build,preview} from 'vite';
import {chromium} from 'playwright';
const out='/tmp/ironsmith-showcase-profile';
if(!process.env.SKIP_BUILD)await build({build:{outDir:out,emptyOutDir:true,copyPublicDir:false},logLevel:'warn'});
for(const name of fs.readdirSync('public'))if(!fs.existsSync(`${out}/${name}`))fs.symlinkSync(`${process.cwd()}/public/${name}`,`${out}/${name}`);
const server=await preview({build:{outDir:out},preview:{host:'127.0.0.1',port:0}}),browser=await chromium.launch();
try{for(let run=0;run<Number(process.env.RUNS||3);run++){
 const context=await browser.newContext();const page=await context.newPage();
 const cdp=await context.newCDPSession(page);await cdp.send('Profiler.enable');await cdp.send('Profiler.start');
 await page.addInitScript(()=>{window.startupProfile={workers:[],tasks:[],requests:[],milestones:[]};const observer=new MutationObserver(()=>{const el=document.querySelector('[data-workspace-shell]');if(el){window.startupProfile.milestones.push({at:performance.now(),kind:'workspace attached',rect:JSON.stringify(el.getBoundingClientRect())});observer.disconnect();requestAnimationFrame(()=>window.startupProfile.milestones.push({at:performance.now(),kind:'workspace frame',rect:JSON.stringify(el.getBoundingClientRect())}));}});observer.observe(document,{childList:true,subtree:true});const Original=window.Worker;window.Worker=class extends Original{constructor(url,options){super(url,options);const entry={url:String(url),start:performance.now()};window.startupProfile.workers.push(entry);this.addEventListener('message',({data:m})=>{if(m.type==='workerDiagnostics'&&m.completed)window.startupProfile.tasks.push({...m.completed,received:performance.now()});if(m.type==='ready')entry.ready=performance.now();});const post=this.postMessage.bind(this);this.postMessage=(m,...rest)=>{window.startupProfile.requests.push({at:performance.now(),type:m.type,method:m.method});return post(m,...rest);};}};});
 const errors=[];page.on('pageerror',e=>errors.push(e.message));const start=Date.now();await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/?showcase=custom`,{waitUntil:'domcontentloaded'});
 await page.waitForSelector('[data-workspace-shell]',{timeout:90000});const totalMs=Date.now()-start;
 const {profile}=await cdp.send('Profiler.stop');const counts=new Map();for(let i=0;i<(profile.samples||[]).length;i++)counts.set(profile.samples[i],(counts.get(profile.samples[i])||0)+(profile.timeDeltas[i]||0));const cpu=profile.nodes.map(n=>({name:n.callFrame.functionName,url:n.callFrame.url,line:n.callFrame.lineNumber,ms:(counts.get(n.id)||0)/1000})).sort((a,b)=>b.ms-a.ms).slice(0,20);
 const result=await page.evaluate(()=>({...window.startupProfile,measures:performance.getEntriesByType('measure').filter(e=>e.name.startsWith('custom-showcase')).map(e=>({name:e.name,ms:e.duration,detail:e.detail})),resources:performance.getEntriesByType('resource').filter(e=>true).map(e=>({url:e.name,start:e.startTime,ms:e.duration,bytes:e.transferSize}))}));console.log(JSON.stringify({run,totalMs,errors,cpu,...result}));await context.close();
}}finally{await browser.close();await server.close();}
