// Register the set's monochrome SVG silhouette against the printing. Compare
// foreground coverage, not rarity color, so bronze/gold/black variants agree.
export function locateSetSymbol(scan, type, template) {
  if(!type||!template)return null;
  const {width,height,data}=scan;
  const rgba=(x,y)=>data.subarray((Math.round(y)*width+Math.round(x))*4,(Math.round(y)*width+Math.round(x))*4+3);
  let best=null;
  for(let h=12;h<=Math.min(40,type.height+4);h+=2) {
    const w=Math.round(h*template.width/template.height);
    if(w<6||w>60)continue;
    for(let y=Math.max(1,Math.floor(type.y-2));y<=Math.min(height-h-2,type.y+type.height-h+3);y+=2)for(let x=Math.floor(width*.82);x<=Math.min(width-w-2,width*.945-w);x+=2) {
      const colors=[];
      for(let i=0;i<8;i++){colors.push(rgba(x-2,y+i*h/8));colors.push(rgba(x+w+1,y+i*h/8));}
      const paper=[0,1,2].map(c=>colors.map(v=>v[c]).sort((a,b)=>a-b)[8]);
      let union=0,intersection=0,foreground=0;
      for(let gy=0;gy<20;gy++)for(let gx=0;gx<20;gx++) {
        const tx=Math.min(template.width-1,Math.floor((gx+.5)*template.width/20));
        const ty=Math.min(template.height-1,Math.floor((gy+.5)*template.height/20));
        const expected=template.data[(ty*template.width+tx)*4+3]>100;
        const color=rgba(x+(gx+.5)*w/20,y+(gy+.5)*h/20);
        const actual=Math.hypot(...paper.map((v,c)=>v-color[c]))>45;
        if(expected)foreground++;
        if(expected||actual)union++;if(expected&&actual)intersection++;
      }
      const score=union?intersection/union:0;
      if(foreground>30&&score>.42&&(!best||score>best.confidence))best={x,y,width:w,height:h,confidence:score};
    }
  }
  return best;
}
