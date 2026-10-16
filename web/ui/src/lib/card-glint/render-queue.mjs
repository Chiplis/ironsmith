// Foreground work jumps ahead of queued warming. One slot stays available for
// it while a background card waits on network assets. Painting stays on the UI
// thread; concurrency overlaps asset loading, not Canvas2D drawing.
export class CardGlintRenderQueue {
  constructor(paint, {concurrency=2, maxBytes=96*1024*1024}={}) {
    Object.assign(this,{paint,concurrency,maxBytes});
    this.entries=new Map(); this.pending=[]; this.running=0; this.backgroundRunning=0; this.bytes=0;
  }
  key(face){return JSON.stringify(face);}
  has(face){return this.entries.has(this.key(face));}
  render(face,{background=false}={}){
    const key=this.key(face), existing=this.entries.get(key);
    if(existing){
      this.entries.delete(key); this.entries.set(key,existing);
      if(!background && existing.background){existing.background=false; this.pump();}
      return existing.promise;
    }
    const entry={key,face,background,bytes:0};
    entry.promise=new Promise((resolve,reject)=>Object.assign(entry,{resolve,reject}));
    this.entries.set(key,entry); this.pending.push(entry);
    this.pump(); return entry.promise;
  }
  pump(){
    while(this.running<this.concurrency){
      let index=this.pending.findIndex(entry=>!entry.background);
      if(index<0 && !this.backgroundRunning) index=this.pending.length?0:-1;
      if(index<0)return;
      const [entry]=this.pending.splice(index,1), wasBackground=entry.background;
      this.running++; if(wasBackground)this.backgroundRunning++;
      Promise.resolve().then(()=>this.paint(entry.face)).then(result=>{
        entry.bytes=result.canvas.width*result.canvas.height*4;
        this.bytes+=entry.bytes; entry.complete=true; this.trim(); entry.resolve(result);
      },error=>{this.entries.delete(entry.key); entry.reject(error);}).finally(()=>{
        this.running--; if(wasBackground)this.backgroundRunning--; this.pump();
      });
    }
  }
  trim(){
    for(const [key,entry] of this.entries){
      if(this.bytes<=this.maxBytes)break;
      // Never evict an in-flight promise: mounts must join background work.
      if(!entry.complete)continue;
      this.entries.delete(key); this.bytes-=entry.bytes;
    }
  }
}
