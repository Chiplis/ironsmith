import {chromium} from 'playwright';import {createServer} from 'vite';
const vite=await createServer({server:{host:'127.0.0.1',port:0,hmr:false,watch:null},logLevel:'error'});await vite.listen();
const browser=await chromium.launch();
const cases=[
 {name:'long activations',text:Array.from({length:25},(_,i)=>`{${i+1}}: Plunder.`).join('\n'),count:20,boardText:'{T}: Add one mana of any color.'},
 {name:'wordy token',text:'{2}: Plunder.',reminder:'Create a 2/2 blue Bird creature token with flying and "Whenever this creature attacks, draw a card."'},
 {name:'granted',text:'Artifacts you control have "{T}: Add one mana of any color."\n{3}: Plunder.',count:25},
 {name:'mixed',types:['Creature'],text:'Flying\nTrample\nVigilance\nWhenever this creature attacks, plunder.\n{2}: Plunder.\nOther creatures you control get +1/+1.',count:25},
 {name:'variable',text:'{X}, Sacrifice X artifacts: Plunder.',count:20},
];
try{for(const c of cases){const page=await browser.newPage();page.setDefaultTimeout(8000);page.on('pageerror',e=>console.log(c.name,'PAGEERROR',e.message));
 try{await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/freeze-probe.html`);await page.waitForFunction(()=>window.freezeSetup);console.log(c.name,'setup');await page.evaluate(c=>window.freezeSetup(c),c);console.log(c.name,'click');await page.getByRole('button',{name:'Inspect card',exact:true}).click();await page.waitForTimeout(1000);await page.getByRole('button',{name:'Toggle layout'}).click();await page.waitForTimeout(1000);await page.getByRole('button',{name:'Close inspector',exact:true}).click();console.log(c.name,'responsive',await page.evaluate(()=>window.freezeCalls));}catch(e){console.log(c.name,'ERROR',e.message);}finally{await page.close();}}}finally{await browser.close();await vite.close();}
