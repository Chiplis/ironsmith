import React, {useState} from 'react';
import {createRoot} from 'react-dom/client';
import {flushSync} from 'react-dom';
import {GameContext} from '../src/context/GameContext.shared';
import {I18nProvider} from '../src/i18n/I18nContext';
import {HoverProvider, useHoverActions} from '../src/context/HoverContext';
import {DragProvider} from '../src/context/DragContext';
import FloatingCardPreview from '../src/components/right-rail/FloatingCardPreview';
import GameCard from '../src/components/cards/GameCard';
import '../src/index.css';

// Controlled, page-local transport. No live Scryfall or game requests.
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const scenarios = [];
const originalFetch = window.fetch.bind(window);
window.fetch = async (input, options) => {
  const url = String(input), decoded = decodeURIComponent(url.replaceAll('+', ' '));
  const s = scenarios.find(s => url.includes(s.slug) || url.includes(s.uuid) || decoded.includes(s.printing.name));
  if (!s) return originalFetch(input, options);
  if (url.includes('api.scryfall.com')) {
    s.apiCalls = (s.apiCalls || 0) + 1;
    await sleep(s.flavorDelay && s.apiCalls === 1 ? s.flavorDelay : s.apiDelay || 0);
    return s.fail ? new Response('', {status:503}) : Response.json(url.includes('/search?') ? {data:[s.printing],has_more:false} : s.printing);
  }
  if (url.includes('/cards/')) {
    // Resolve the image locally; delayed API calls exercise frame metadata,
    // independently of the current default-printing search policy.
    const printing = {...s.printing,standard_printing:true,frame:'2015'};
    if (s.flavorDelay) delete printing.flavor_text;
    return Response.json({scryfall:printing});
  }
  return originalFetch(input, options);
};
const src = Object.getOwnPropertyDescriptor(HTMLImageElement.prototype, 'src');
const requests = new WeakMap();
Object.defineProperty(HTMLImageElement.prototype, 'src', {...src, set(url) {
  const s = scenarios.find(s => String(url).includes(s.uuid));
  if (!s) {src.set.call(this, url);return;}
  requests.set(this, {url,s});
  const normal = String(url).includes('/normal/');
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="488" height="${normal ? 684 : 356}"><rect width="488" height="684" fill="#171612"/><rect x="24" y="28" width="440" height="625" fill="#9c7848"/><rect x="48" y="70" width="390" height="300" fill="#b4d2f0"/><rect x="48" y="416" width="390" height="185" fill="#ece0ca"/></svg>`;
  if (normal && s.fail) {queueMicrotask(() => this.onerror?.(new Event('error'))); return;}
  if (normal && s.textureDelay) {
    s.textureStarted ??= performance.now();
    setTimeout(() => src.set.call(this, `data:image/svg+xml,${encodeURIComponent(svg)}`), s.textureDelay);
  } else src.set.call(this, `data:image/svg+xml,${encodeURIComponent(svg)}`);
}});
const decode = HTMLImageElement.prototype.decode;
const artWaits = new Map();
HTMLImageElement.prototype.decode = async function() {
  const s = requests.get(this)?.s;
  if (s?.artDelay) {
    if (!artWaits.has(s)) artWaits.set(s, sleep(s.artDelay));
    await artWaits.get(s);
  }
  if (s?.fail) throw new Error('Fixture image unavailable');
  return decode.call(this);
};
const loadFont = document.fonts.load.bind(document.fonts);
let fontDelay = 0;
document.fonts.load = async (...args) => {await sleep(fontDelay);return loadFont(...args);};
const check = (condition, message) => {if (!condition) throw new Error(message);};
async function until(predicate) {
  const start = performance.now();
  while (!predicate()) {if (performance.now() - start > 6000) throw new Error('Timed out waiting for frame');await sleep(16);}
}
export default function Fixture() {
  const {hoverCard, clearHover} = useHoverActions();
  const [cards, setCards] = useState([]), [results, setResults] = useState([]), [running, setRunning] = useState(false);
  const [fieldCard, setFieldCard] = useState(false);
  const run = async () => {
    setRunning(true);setResults([]);setFieldCard(false);
    try {
      await Promise.all(['400 100px "Goudy Medieval"','400 100px "MPlantin"','italic 400 100px "MPlantin"'].map(font => loadFont(font)));
      for (const [label, delays] of [
        ['fast preparation respects hover delay', {}],
        ['art appears while printing metadata loads', {apiDelay:900}],
        ['art appears while initial card details load', {detailsDelay:900}],
        ['art appears while flavor text loads', {flavorDelay:900}],
        ['texture calculation starts during delay', {textureDelay:900}],
        ['slow artwork decode stays hidden', {artDelay:900}],
        ['art appears while fonts load', {fontDelay:900}],
        ['failed assets show a settled fallback', {fail:true}],
      ]) {
        flushSync(() => {clearHover();setCards([]);});await sleep(270);
        const id = scenarios.length + 1, uuid = `aaaaaaaa-bbbb-cccc-dddd-${String(id).padStart(12,'0')}`;
        const name = `Readiness fixture ${id}`, slug = name.toLowerCase().replaceAll(' ', '-');
        const art = `https://cards.scryfall.io/art_crop/front/a/b/${uuid}.jpg`;
        const card = {id,name,type_line:'Artifact Creature — Thopter',oracle_text:'Flying',mana_cost:'{0}',power:0,toughness:2,zone:'Battlefield'};
        const printing = {...card,frame:'1997',image_uris:{art_crop:art,normal:art.replace('/art_crop/','/normal/')},flavor_text:'A fully prepared frame.'};
        const scenario = {slug,uuid,printing,...delays};scenarios.push(scenario);fontDelay = delays.fontDelay || 0;
        flushSync(() => setCards([card]));
        const started = performance.now();flushSync(() => hoverCard(id));
        const preview = () => document.querySelector('[data-card-hover-preview]');
        const visible = () => preview()?.dataset.visible === 'true';
        await sleep(350);check(!visible(), `${label}: appeared before hover delay`);
        if (delays.apiDelay || delays.detailsDelay || delays.flavorDelay || delays.textureDelay || delays.artDelay || delays.fontDelay) {
          await sleep(260);
          const stage = preview()?.querySelector('.interactive-card-frame-stage');
          check(stage?.dataset.renderReady === 'false', `${label}: unfinished frame revealed`);
          check(stage.inert, `${label}: unfinished frame accepts interaction`);
          if (!delays.artDelay) {
            check(visible(), `${label}: artwork blocked by frame preparation`);
            check(preview().querySelector('.card-frame-art-preview'), `${label}: artwork missing`);
          } else check(!visible(), `${label}: undecoded artwork revealed`);
        }
        await until(() => visible() && preview().querySelector('.interactive-card-frame-stage')?.dataset.renderReady === 'true');
        const elapsed = performance.now() - started, stage = preview().querySelector('.interactive-card-frame-stage');
        check(stage.dataset.renderReady === 'true' && stage.dataset.printingReady === 'true', `${label}: visible before readiness`);
        check(!stage.inert && !preview().inert, `${label}: ready card remains inert`);
        const artwork = preview().querySelector('.card-frame-art-preview');
        if (artwork) {
          check(artwork.dataset.frameReady === 'true', `${label}: artwork did not dissolve`);
          await sleep(280);
          check(getComputedStyle(artwork).opacity === '0', `${label}: artwork remains visible`);
          check(getComputedStyle(stage).opacity === '1', `${label}: frame did not finish fading`);
        }
        check(elapsed >= 490, `${label}: hover delay shortened`);
        if (delays.textureDelay) {
          check(scenario.textureStarted - started < 400, 'Texture preparation started after hover delay');
        }
        if (!delays.fail) {
          check(stage.dataset.cardEra === 'retro', 'Default font leaked into visible frame');
          check(stage.querySelector('[aria-label="Flavor text"]'), 'Flavor missing at reveal');
          if (stage.dataset.frameMode === 'masked') {
            check(parseFloat(stage.querySelector('[data-fit-text]').style.getPropertyValue('--card-fitted-rules-font-size')) > 0, 'Text not fitted at reveal');
          } else {
            check(stage.querySelector('.original-card-fallback'), 'Missing original-image fallback');
            check(!stage.querySelector('.interactive-card-frame'), 'Synthetic frame was rendered');
          }
        }
        setResults(items => [...items, `PASS ${label} (${Math.round(elapsed)} ms)`]);
      }
      flushSync(() => clearHover());await sleep(270);
      flushSync(() => hoverCard(cards[0]?.id || scenarios.length));await sleep(100);
      flushSync(() => clearHover());await sleep(600);
      check(document.querySelector('[data-card-hover-preview]').dataset.visible === 'false', 'Abandoned hover reopened');
      setResults(items => [...items,'PASS cancelled hover stays closed']);
      const id = scenarios.length + 1, uuid = `aaaaaaaa-bbbb-cccc-dddd-${String(id).padStart(12,'0')}`;
      const art = `https://cards.scryfall.io/art_crop/front/a/b/${uuid}.jpg`;
      const card = {id,name:'Background frame fixture',type_line:'Artifact',oracle_text:'Flying',mana_cost:'{0}'};
      scenarios.push({uuid,slug:'background-frame-fixture',apiDelay:400,printing:{...card,frame:'1997',image_uris:{art_crop:art,normal:art.replace('/art_crop/','/normal/')}}});
      flushSync(() => {setCards([card]);setFieldCard(true);});
      const smallFrame = () => document.querySelector('.battlefield-prepared-frame .interactive-card-frame-stage');
      await until(() => smallFrame()?.dataset.renderReady === 'true');
      check(!document.querySelector('.battlefield-prepared-frame .card-frame-art-preview'), 'Small frame flashes a second art preview');
      check(document.querySelector('.battlefield-prepared-frame').inert, 'Small frame accepts inspector interactions');
      flushSync(() => hoverCard(id));
      await until(() => document.querySelector('[data-card-hover-preview] .interactive-card-frame-stage')?.dataset.renderReady === 'true');
      check(document.querySelector('[data-card-hover-preview] .interactive-card-frame-stage').dataset.frameReused === 'true', 'Hover did not reuse background frame');
      check(!document.querySelector('[data-card-hover-preview] .card-frame-art-preview'), 'Prepared hover switched artwork');
      setResults(items => [...items,'PASS battlefield prepares before hover and shares its finished frame','ALL CHECKS PASSED']);
    } catch (error) {setResults(items => [...items, `FAIL ${error.message}`]);}
    finally {fontDelay = 0;setRunning(false);}
  };
  return <GameContext.Provider value={{state:{players:[{id:0,battlefield:cards}],perspective:0},game:{objectDetails:async id => {
    const s = scenarios.find(s => s.printing.id === Number(id));
    await sleep(s?.detailsDelay || 0);
    return s?.printing || null;
  }}}}>
    <button disabled={running} onClick={run}>Run readiness checks</button>
    <pre aria-label="Readiness results">{results.join('\n')}</pre>
    {cards.map(card => fieldCard
      ? <GameCard key={card.id} card={card} variant="battlefield" battlefieldVisualMode="portrait" sourceImageUrl={scenarios.at(-1).printing.image_uris.normal} style={{width:80,height:110,marginTop:250}} />
      : <div key={card.id} className="game-card battlefield-row-card" data-object-id={card.id} style={{width:80,height:110,marginTop:250}}>{card.name}</div>)}
    <FloatingCardPreview />
  </GameContext.Provider>;
}
createRoot(document.getElementById('root')).render(<I18nProvider><HoverProvider><DragProvider><Fixture /></DragProvider></HoverProvider></I18nProvider>);
