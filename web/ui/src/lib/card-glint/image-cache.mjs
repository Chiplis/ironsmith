// Keep upstream's URL-specific SVG fixes and extra historical bitmap aliases.
// The upstream static painter normally decodes and closes these for every face.
export class CardGlintImageCache {
  constructor(load,{maxBytes=64*1024*1024}={}){
    Object.assign(this,{load,maxBytes}); this.entries=new Map(); this.bytes=0;
  }
  async acquire(url){
    let entry=this.entries.get(url);
    if(entry){this.entries.delete(url);this.entries.set(url,entry);}
    else{
      entry={refs:0,bytes:0}; this.entries.set(url,entry);
      entry.promise=Promise.resolve().then(()=>this.load([url],new AbortController().signal)).then(lease=>{
        entry.lease=lease;
        entry.bytes=[...new Set(lease.images.values())].reduce((sum,image)=>sum+image.width*image.height*4,0);
        this.bytes+=entry.bytes;return lease;
      },error=>{this.entries.delete(url);throw error;});
    }
    entry.refs++;
    try{
      const lease=await entry.promise;
      return {images:lease.images,release:()=>{entry.refs--;this.trim();}};
    }catch(error){entry.refs--;throw error;}
  }
  trim(){
    for(const [url,entry] of this.entries){
      if(entry.refs || !entry.lease)continue;
      if(this.bytes<=this.maxBytes && entry.bytes)continue;
      this.entries.delete(url);this.bytes-=entry.bytes;entry.lease.dispose();
    }
  }
  async images(urls,signal){
    const pending=[...new Set(urls.filter(Boolean))], leases=[];
    let index=0;
    const settled=await Promise.allSettled(Array.from({length:Math.min(4,pending.length)},async()=>{
      while(index<pending.length){signal.throwIfAborted();leases.push(await this.acquire(pending[index++]));}
    }));
    const release=()=>leases.forEach(lease=>lease.release());
    const failure=settled.find(result=>result.status==='rejected');
    if(failure || signal.aborted){release();throw failure?.reason ?? signal.reason;}
    const images=new Map();
    for(const lease of leases)for(const [url,image] of lease.images)images.set(url,image);
    return {images,dispose:release};
  }
}
