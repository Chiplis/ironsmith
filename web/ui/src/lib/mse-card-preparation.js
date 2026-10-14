import {isHiddenCardName,resolveScryfallImageUrl,resolveScryfallPrintingMetadata,resolveScryfallSetIcon} from './scryfall.js';
import {cardFrameSnapshot} from './card-frame-snapshot.mjs';
import {mseFrameRequest} from './mse/frame-request.mjs';
import {mseDefaultFrameKey} from './mse/default-frames.mjs';

// Cards anyone can inspect: every player's battlefield first, then their
// graveyards, exiles and command zones. Face-down and hidden cards are skipped.
export function publicZoneCards(state){
  const players=state?.players||[],cards=[];
  const add=card=>{if(card?.name&&!card.face_down&&!isHiddenCardName(card.name))cards.push(card);};
  for(const player of players)for(const card of player.battlefield||[])add(card);
  for(const zone of ['graveyard_cards','exile_cards','command_cards'])for(const player of players)for(const card of player[zone]||[])add(card);
  return cards;
}

const idle=callback=>globalThis.requestIdleCallback?globalThis.requestIdleCallback(callback,{timeout:3000}):setTimeout(callback,100);
const PREPARED_LIMIT=2000;

// Prepares public cards' frames at idle time, one at a time: their printing,
// set icon, artwork, template assets and stand-in frame are fetched, and the
// frame is rendered once (cached by the renderer). Inspecting one then shows
// its real frame almost at once instead of a stand-in.
export class MseCardPreparer{
  constructor({session,defaultFrames=null,defaultStyle,schedule=idle,
    resolveImage=resolveScryfallImageUrl,resolvePrinting=resolveScryfallPrintingMetadata,resolveSetIcon=resolveScryfallSetIcon}){
    Object.assign(this,{session,defaultFrames,defaultStyle,schedule,resolveImage,resolvePrinting,resolveSetIcon});
    this.queue=[];this.prepared=new Set();this.running=false;this.disposed=false;
  }
  // The latest public cards replace the queue; already prepared cards are skipped.
  update(cards){
    if(this.disposed)return;
    const seen=new Set();
    this.queue=[];
    for(const card of cards){
      const key=cardFrameSnapshot(card);
      if(seen.has(key)||this.prepared.has(key))continue;
      seen.add(key);this.queue.push({key,card});
    }
    this.pump();
  }
  pump(){
    if(this.running||this.disposed||!this.queue.length)return;
    this.running=true;
    this.schedule(async()=>{
      const job=this.queue.shift();
      try{if(job&&!this.disposed)await this.prepare(job.card);}
      catch{/* the card's own surface reports any failure when shown */}
      finally{
        if(job){
          this.prepared.add(job.key);
          if(this.prepared.size>PREPARED_LIMIT)this.prepared.delete(this.prepared.values().next().value);
        }
        this.running=false;this.pump();
      }
    });
  }
  async prepare(card){
    const imageUrl=await this.resolveImage(card.name,'normal');
    const printing=imageUrl?await this.resolvePrinting(imageUrl):null;
    const setSymbol=printing?.set?await this.resolveSetIcon(printing.set):null;
    if(this.disposed)return;
    const snapshot=JSON.parse(cardFrameSnapshot(card));
    const {request}=mseFrameRequest(snapshot,{imageUrl,requestedStyle:card.mse_style,defaultStyle:this.defaultStyle,
      printing,setSymbol,availableStyles:this.session.styles});
    if(!request.style)return;
    this.defaultFrames?.request(request.style,mseDefaultFrameKey(snapshot));
    await this.session.renderer.render(request);
  }
  dispose(){this.disposed=true;this.queue=[];}
}
