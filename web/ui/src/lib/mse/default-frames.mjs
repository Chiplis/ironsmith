import {selectMsePresentation} from './style-selection.mjs';
import {cardArtColors} from '../card-art-colors.js';
import {cssColor} from './symbol-painter.mjs';

// Blank MSE frames stand in for a card while its own render is in flight. A
// frame's look depends on its colors and on being a land or an artifact, so
// one blank card per combination covers every card of that style.
const CODES='WUBRG';
const NAMES={white:'W',blue:'U',black:'B',red:'R',green:'G'};
const comboColors=Array.from({length:32},(_,mask)=>[...CODES].filter((_,bit)=>mask&(1<<bit)).join(''))
  .sort((a,b)=>a.length-b.length||CODES.indexOf(a[0])-CODES.indexOf(b[0]));
// Warmed in likely-to-be-needed order: mono colors first, lands and colorless
// artifacts next, then every multicolor combination.
export const MSE_DEFAULT_FRAME_WARMUP=[
  ...comboColors.filter(c=>c.length<=1).map(colors=>({colors,kind:''})),
  {colors:'',kind:'artifact'},
  ...comboColors.filter(c=>c.length<=1).map(colors=>({colors,kind:'land'})),
  ...comboColors.filter(c=>c.length>1).map(colors=>({colors,kind:''})),
];

const colorCodes=value=>{
  if(!Array.isArray(value))return null;
  const found=new Set(value.map(entry=>NAMES[String(entry).toLowerCase()]??String(entry).toUpperCase()));
  return [...CODES].filter(code=>found.has(code)).join('');
};

// Nonland frames follow the card's colors. Live objects carry them in
// characteristic_signature; hand cards and inspector details carry neither
// that nor colors, so their mana cost decides (cardArtColors). Land frames
// follow the mana they make: color identity, or the symbols their rules add.
export function mseDefaultFrameKey(snapshot,faceIndex=0){
  const faces=snapshot?.card_faces;
  const face=Array.isArray(faces)&&faces.length&&!['split','adventure'].includes(snapshot.layout)?faces[faceIndex]??faces[0]:snapshot;
  const types=[face?.type_line,snapshot?.type_line,...(Array.isArray(snapshot?.card_types)?snapshot.card_types:[])].map(value=>String(value??'')).join(' ');
  const kind=/\bLand\b/i.test(types)?'land':/\bArtifact\b/i.test(types)?'artifact':'';
  if(kind==='land'){
    const identity=colorCodes(snapshot?.color_identity??snapshot?.colorIdentity);
    const rules=String(face?.oracle_text??snapshot?.oracle_text??'');
    const produced=[...rules.matchAll(/\bAdd\b[^.\n]*/g)].flatMap(clause=>[...clause[0].matchAll(/\{([^}]+)\}/g)].flatMap(symbol=>symbol[1].split('/')));
    return {colors:identity||colorCodes(produced)||'',kind};
  }
  const live=face===snapshot?snapshot:{...face,characteristic_signature:snapshot?.characteristic_signature};
  return {colors:cardArtColors(live).join(''),kind};
}

const STAND_IN_FIELDS=['name','casting_cost','image','type','text','pt'];
const frameId=(style,{colors,kind})=>`${style}|${colors}|${kind}`;
// Saga, planeswalker and battle templates only lay out their own frame for
// cards of that type, so the blank card takes the type its style is for.
function blankCard({colors,kind},style=''){
  const base=kind==='land'?'Land':kind==='artifact'?'Artifact':'';
  const special=/saga/i.test(style)?'Enchantment — Saga':/planeswalker/i.test(style)?'Planeswalker':/battle/i.test(style)?'Battle':null;
  const type=special?(base?`${base} ${special}`:special):base||'Sorcery';
  return {name:'',mana_cost:'',type_line:type,type_line_display:'',colors:[...colors]};
}
// Colors without a mana cost would show a color indicator dot that cards
// with a cost never have. The footer stays blank rather than "C 0001".
const BLANK_FIELDS={indicator:'colorless'};
const BLANK_SET={automatic_card_numbers:'no',rarity_codes:'no'};
const idle=callback=>globalThis.requestIdleCallback?globalThis.requestIdleCallback(callback,{timeout:2000}):setTimeout(callback,50);

export class MseDefaultFrames{
  constructor({renderer,fonts=null,defaultStyle='auto',availableStyles,schedule=idle,encode=canvas=>canvas.convertToBlob({type:'image/webp',quality:0.92})}){
    Object.assign(this,{renderer,fonts,schedule,encode});
    this.defaultStyle=selectMsePresentation(blankCard({colors:'',kind:''}),{defaultStyle,availableStyles}).style;
    this.entries=new Map();this.urgent=[];this.background=[];this.listeners=new Set();this.rendering=false;this.scheduled=false;this.disposed=false;
  }
  subscribe=listener=>{this.listeners.add(listener);return ()=>this.listeners.delete(listener);};
  // Pure lookup for rendering: the exact style first, then the default style's
  // frame for the same combination, then its nonland frame of the same colors.
  lookup(style,key){
    const candidates=[[style,key],[this.defaultStyle,key],[this.defaultStyle,{colors:key.colors,kind:''}]];
    for(const [s,k] of candidates){const frame=s&&this.entries.get(frameId(s,k))?.frame;if(frame)return frame;}
    return null;
  }
  // A card that needs a frame not yet rendered moves ahead of the warm-up.
  request(style,key){if(style)this.enqueue(style,key,this.urgent);}
  warm(keys=MSE_DEFAULT_FRAME_WARMUP){if(this.defaultStyle)for(const key of keys)this.enqueue(this.defaultStyle,key,this.background);}
  enqueue(style,key,queue){
    const id=frameId(style,key);
    if(this.disposed||this.entries.has(id))return;
    this.entries.set(id,{frame:null});queue.push({id,style,key});this.pump();
  }
  // One render at a time. Urgent frames start at once; warm-up waits for idle.
  pump(){
    if(this.rendering||this.disposed)return;
    if(this.urgent.length){this.renderNext();return;}
    if(this.background.length&&!this.scheduled){
      this.scheduled=true;
      this.schedule(()=>{this.scheduled=false;if(!this.rendering&&!this.disposed)this.renderNext();});
    }
  }
  async renderNext(){
    const job=this.urgent.shift()??this.background.shift();
    if(!job)return;
    this.rendering=true;
    try{
      const result=await this.renderer.render({style:job.style,snapshot:blankCard(job.key,job.style),adapter:{styling:{},faceIndex:null,nativeCard:BLANK_FIELDS,set:BLANK_SET}},{cache:false});
      const [blob,fields]=await Promise.all([this.encode(result.canvas),this.fieldLayout(result.context)]);
      if(this.disposed)return;
      this.entries.set(job.id,{frame:{url:URL.createObjectURL(blob),width:result.width,height:result.height,fields}});
      for(const listener of this.listeners)listener();
    }catch(error){
      // A missing frame only means the card keeps the simple fallback.
      if(!this.disposed)console.warn('[MSE] Default frame failed',{frame:job.id,error});
    }finally{
      this.rendering=false;this.pump();
    }
  }
  // Where the frame puts the card's name, cost, art, type, rules and P/T, so a
  // stand-in can lay the card's own art and text over the blank frame.
  async fieldLayout(context){
    const fields={};
    if(!context)return fields;
    for(const name of STAND_IN_FIELDS){
      if(!context.template?.layouts?.has(name))continue;
      try{
        // Text boxes a frame hides (e.g. a Saga's rules box) stay out; art
        // is placed even when a blank card's image field reports hidden.
        if(name!=='image'&&!context.fieldVisible(name))continue;
        const value=key=>Number(context.property(name,key));
        const rect={left:value('left'),top:value('top'),width:value('width'),height:value('height'),angle:value('angle')||0};
        if(![rect.left,rect.top,rect.width,rect.height].every(Number.isFinite)||rect.width<=0||rect.height<=0)continue;
        if(name!=='image'){
          const font=context.font(name);
          rect.size=Number(font.size);rect.color=font.color?cssColor(font.color):null;
          // Load the face so the stand-in can use it, without failing the frame.
          if(this.fonts){await this.fonts.prepare(font).catch(()=>{});rect.face=this.fonts.cssFace(font);}
        }
        fields[name]=rect;
      }catch{/* a missing field only leaves that part of the stand-in out */}
    }
    return fields;
  }
  dispose(){
    this.disposed=true;this.urgent=[];this.background=[];this.listeners.clear();
    for(const entry of this.entries.values())if(entry.frame)URL.revokeObjectURL(entry.frame.url);
    this.entries.clear();
  }
}
