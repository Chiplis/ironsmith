import test from 'node:test';
import process from 'node:process';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

const settled = page => page.waitForFunction(() => {
  const stage = document.querySelector('[data-card-glint-live]');
  return ['live','static','error'].includes(stage?.dataset.cardGlintLive) && !stage.dataset.cardGlintRendering;
}, undefined, {timeout:150000});

test('Build previews finish, print effects and printing styles with card-glint', {timeout:600000}, async () => {
  const vite=await createServer({server:{host:'127.0.0.1',port:0,hmr:false,watch:null},logLevel:'error'});
  await vite.listen();
  const browser=await chromium.launch({args:process.env.CARD_GLINT_TEST_ANGLE ? [`--use-angle=${process.env.CARD_GLINT_TEST_ANGLE}`] : []});
  try {
    const page=await browser.newPage({viewport:{width:500,height:1100}}), errors=[];
    // Exercise the actual finish renderer even on headless software GPUs.
    // Build's normal capability fallback is covered by build-performance.
    await page.addInitScript(() => {
      const getContext = HTMLCanvasElement.prototype.getContext;
      HTMLCanvasElement.prototype.getContext = function(type, options) {
        return getContext.call(this, type, type === 'webgl2'
          ? { ...options, failIfMajorPerformanceCaveat: false } : options);
      };
    });
    page.setDefaultTimeout(150000);
    page.on('pageerror',error=>{errors.push(error.message);console.error(error.stack);});
    page.on('console',message=>{if(message.type()==='error') console.error(message.text());});
    const url=`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint-build.html`;
    await page.goto(url,{waitUntil:'domcontentloaded',timeout:200000});
    await settled(page);
    await page.getByRole('checkbox',{name:'Preview finish effects',exact:true}).click();
    await settled(page);
    const stage=page.locator('[data-card-glint-live]');
    assert.equal(await stage.getAttribute('data-card-glint-live'),'live','WebGL renderer paints the preview');
    const pressed=async(group,name)=>String(await page.getByRole('combobox',{name:group,exact:true}).locator('option:checked').textContent() === name);
    assert.equal(await pressed('Finish','Shiny'),'true','Shiny is the default finish');
    assert.equal(await pressed('Printing style','Current printing'),'true','the current printing is the default style');
    assert.equal(await page.getByRole('combobox',{name:'Print effects',exact:true}).inputValue(),'on','print effects default on');

    const card=page.locator('.card-glint-live__tilt');
    const shot=async()=>{await page.mouse.move(0,0);await page.waitForTimeout(300);return card.screenshot();};
    const shiny=await shot();
    await page.screenshot({path:'test-results/card-glint-build-shiny.png'});

    await page.getByRole('combobox',{name:'Print effects',exact:true}).selectOption('off');
    assert.notDeepEqual(await shot(),shiny,'print effects change the printed surface');
    await page.getByRole('combobox',{name:'Print effects',exact:true}).selectOption('on');

    await page.getByRole('combobox',{name:'Finish',exact:true}).selectOption({label:'Prismatic'});
    const prismatic=await shot();
    assert.notDeepEqual(prismatic,shiny,'finish changes the painted foil');
    const box=await card.boundingBox();
    await page.mouse.move(box.x+box.width*.9,box.y+box.height*.15);await page.waitForTimeout(300);
    assert.notDeepEqual(await card.screenshot(),prismatic,'pointer moves the foil highlight');
    assert.equal(await card.evaluate(el => getComputedStyle(el).transform), 'none', 'hover must not warp the card text');
    for (const quality of ['off', 'low', 'high']) {
      await page.evaluate(async q => {
        const {setGraphicsQuality} = await import('/src/hooks/useGraphicsQuality.js');
        setGraphicsQuality(q);
      }, quality);
      await page.waitForFunction(q => document.querySelector('[data-card-glint-live]')?.dataset.cardGlintLive === (q === 'high' ? 'live' : 'static'), quality);
      await settled(page);
      assert.equal(await card.evaluate(el => getComputedStyle(el).transform), 'none');
    }

    await page.getByRole('combobox',{name:'Printing style',exact:true}).selectOption({label:'Fourth Edition'});
    await settled(page);
    const fourth=await shot();
    assert.notDeepEqual(fourth,prismatic,'a set style repaints the frame');
    await page.screenshot({path:'test-results/card-glint-build-4ed.png'});

    await page.getByRole('combobox',{name:'Printing style',exact:true}).selectOption({label:'Future Sight'});
    await page.getByRole('combobox',{name:'Frame',exact:true}).selectOption({label:'Futureshifted'});
    await settled(page);
    assert.notDeepEqual(await shot(),fourth,'a style frame repaints the card');
    await page.screenshot({path:'test-results/card-glint-build-futureshifted.png'});

    await page.getByRole('button',{name:'Change border color'}).first().click();
    assert.equal(await pressed('Border','Black'),'true');
    await page.getByRole('button',{name:'Change rarity'}).click();
    assert.equal(await pressed('Rarity','Common'),'true');

    // Text edits keep the last card on screen until the next revision is ready.
    await page.evaluate(()=>window.setBuildFace({name:'Renamed Adept'}));
    await page.getByRole('img',{name:/Renamed Adept/}).waitFor();
    await settled(page);
    assert.equal(await stage.getAttribute('data-card-glint-live'),'live');
    assert.deepEqual(errors,[]);

    // In game, only the copy added from Build uses the live renderer and its choices.
    await page.goto(`${url}?view=table`,{waitUntil:'domcontentloaded',timeout:200000});
    const chosen=page.locator('[data-table-card="501"]'), plain=page.locator('[data-table-card="502"]');
    await chosen.locator('[data-card-glint-finish="prismatic"]').waitFor({timeout:150000});
    await plain.locator('[data-card-glint-frame="ready"]').waitFor({timeout:150000});
    assert.equal(await plain.locator('.card-glint-surface__live').count(),0,'other copies keep the static face');
    assert.notDeepEqual(await chosen.screenshot(),await plain.screenshot());
    await page.screenshot({path:'test-results/card-glint-build-table.png'});
    assert.deepEqual(errors,[]);

    // Without the WebGL renderer the static Canvas2D face still shows the frame.
    await page.route('**/card-glint/**/browser.js',route=>route.abort());
    await page.goto(url,{waitUntil:'domcontentloaded',timeout:200000});
    await settled(page);
    await page.getByRole('checkbox',{name:'Preview finish effects',exact:true}).click();
    await settled(page);
    assert.equal(await stage.getAttribute('data-card-glint-live'),'static');
    const opaque=await page.locator('.card-glint-live__canvas:not([hidden])').evaluate(canvas=>{
      const data=canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data;
      let count=0; for(let i=3;i<data.length;i+=4) if(data[i]>0) count++;
      return count/(canvas.width*canvas.height);
    });
    assert.ok(opaque>.8,'static fallback paints the frame');
  } finally {await browser.close();await vite.close();}
});

test('Build prints shuffled mana and type definitions identically', {timeout:180000}, async () => {
  const vite=await createServer({server:{host:'127.0.0.1',port:0,hmr:false,watch:null},logLevel:'error'});
  await vite.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:500,height:1100}}), errors=[];
    page.on('pageerror',error=>errors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint-build.html`,{waitUntil:'domcontentloaded'});
    await page.waitForFunction(()=>Boolean(window.setBuildFace));
    const setFace=async patch=>{
      await page.evaluate(value=>window.setBuildFace(value),patch);
      await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
      await settled(page);
      await page.getByRole('img',{name:/Ordering Sentinel: Legendary Artifact Creature — Golem/}).first().waitFor();
    };
    await setFace({name:'Ordering Sentinel',manaCost:'{2}{G}{G}{W}',supertypes:['Legendary'],
      cardTypes:['Artifact','Creature'],subtypes:['Golem']});
    const card=page.locator('.card-glint-live__tilt');
    const canonical=await card.screenshot();
    await setFace({manaCost:'{W}{G}{2}{G}',cardTypes:['Creature','Artifact']});
    assert.deepEqual(await card.screenshot(),canonical,'shuffled input paints the same type line and mana symbols');
    await page.screenshot({path:'test-results/card-print-order.png'});
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await vite.close();}
});
