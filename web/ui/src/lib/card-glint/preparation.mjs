import {isHiddenCardName,resolveScryfallImageUrl,resolveScryfallLocalizedImageUrl,resolveScryfallPrintingMetadata} from '../scryfall.js';
import {cardFrameSnapshot,cardFrameFaceIndex} from '../card-frame-snapshot.mjs';
import {cardGlintFace} from './card.mjs';
import {renderCardGlint,hasCardGlintFrame} from './session.mjs';

// Only our visible hand and public battlefield identities are eligible. A
// compact board or a scan-based hand still warms its full inspection frames.
export function cardGlintZoneCards(state){
  const players=state?.players ?? [], cards=[];
  const add=card=>{if(card?.name && !card.face_down && !card.source_kind && !isHiddenCardName(card.name))cards.push(card);};
  const own=state?.perspective == null ? null : players.find((player,index)=>String(player.id ?? player.index ?? index)===String(state.perspective));
  for(const card of own?.hand_cards ?? [])add(card);
  for(const player of players)for(const card of player.battlefield ?? [])add(card);
  return cards;
}

function displayedImage(card){
  if(typeof document==='undefined' || card.id==null)return null;
  return [...document.querySelectorAll('[data-object-id][data-card-image-url]')]
    .find(node=>node.dataset.objectId===String(card.id) && !node.classList.contains('stack-card'))?.dataset.cardImageUrl || null;
}
export async function prepareCardGlintFace(card,{locale='en',resolveImage,resolvePrinting=resolveScryfallPrintingMetadata}={}){
  let imageUrl=resolveImage ? await resolveImage(card) : displayedImage(card);
  if(!resolveImage && !imageUrl){
    imageUrl=locale==='en' ? null : await resolveScryfallLocalizedImageUrl(card.name,locale,'normal');
    imageUrl ||= await resolveScryfallImageUrl(card.name,'normal');
  }
  const printing=imageUrl ? await resolvePrinting(imageUrl) : null;
  return cardGlintFace(JSON.parse(cardFrameSnapshot(card)),{imageUrl,printing,faceIndex:cardFrameFaceIndex(card,imageUrl)});
}

export class CardGlintPreparer {
  constructor({prepare=prepareCardGlintFace,render=renderCardGlint,has=hasCardGlintFrame,
    schedule=callback=>setTimeout(callback,0),concurrency=2}={}){
    Object.assign(this,{prepare,render,has,schedule,concurrency});
    this.desired=new Map(); this.prepared=new Map(); this.running=new Set(); this.queue=[]; this.disposed=false;
  }
  update(cards){
    if(this.disposed)return;
    this.desired=new Map(cards.map(card=>[cardFrameSnapshot(card),card]));
    for(const key of this.prepared.keys())if(!this.desired.has(key))this.prepared.delete(key);
    this.queue=[...this.desired].filter(([key])=>!this.running.has(key) &&
      (!this.prepared.has(key) || !this.has(this.prepared.get(key))));
    this.pump();
  }
  pump(){
    while(!this.disposed && this.running.size<this.concurrency && this.queue.length){
      const [key,card]=this.queue.shift(); this.running.add(key);
      this.schedule(async()=>{
        try{
          if(this.disposed || !this.desired.has(key))return;
          const face=await this.prepare(card);
          if(this.disposed || !this.desired.has(key))return;
          await this.render(face,{background:true});
          if(!this.disposed && this.desired.has(key))this.prepared.set(key,face);
        }catch{/* Keep the visible fallback usable; retry on a later state update. */}
        finally{this.running.delete(key);this.pump();}
      });
    }
  }
  dispose(){this.disposed=true;this.queue=[];this.desired.clear();this.prepared.clear();}
}
