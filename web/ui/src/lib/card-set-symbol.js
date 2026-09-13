// Register the set's monochrome SVG silhouette against the printing. Compare
// foreground coverage, not rarity color, so bronze/gold/black variants agree.
export function locateSetSymbol(scan, type, template) {
  if(!type||!template)return null;
  const {width,height,data}=scan;
  const rgba=(x,y)=>data.subarray((Math.round(y)*width+Math.round(x))*4,(Math.round(y)*width+Math.round(x))*4+3);
  let best=null;
  for(let h=Math.max(12,Math.round(type.height*.5));h<=Math.min(40,type.height+4);h+=2) {
    const w=Math.round(h*template.width/template.height);
    if(w<6||w>60)continue;
    // The complete symbol sits on the type bar's centerline. Tiny fragments
    // along a bevel can otherwise outscore the full boxed silhouette.
    for(let y=Math.max(1,Math.floor(type.y-2));y<=Math.min(height-h-2,type.y+type.height-h+3);y+=2)
    for(let x=Math.floor(width*.75);x<=Math.min(width-w-2,width*.945-w);x+=2) {
      if(Math.abs(y+h/2-(type.y+type.height/2))>type.height*.12)continue;
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
  if(!best)return null;
  // The template seeds the search; recover the complete connected enclosure
  // from the scan. A partial letter/edge match must never become the boundary
  // for erasing a type line (boxed core-set logos contain both).
  const samples=[];
  // Sample an area: a single baseline can be mostly dark type lettering
  // and invert foreground/background on a long translated type line.
  for(let row=0;row<9;row++)for(let i=0;i<21;i++)samples.push(rgba(type.x+type.width*(.55+i*.15/20),type.y+type.height*(.2+row*.6/8)));
  const paper=[0,1,2].map(c=>samples.map(v=>v[c]).sort((a,b)=>a-b)[Math.floor(samples.length/2)]);
  const x0=Math.floor(width*.75),x1=Math.floor(width*.95);
  const y0=Math.ceil(type.y+1),y1=Math.floor(type.y+type.height-1);
  const ink=new Set();
  for(let y=y0;y<y1;y++)for(let x=x0;x<x1;x++)if(Math.hypot(...paper.map((v,c)=>v-rgba(x,y)[c]))>55)ink.add(y*width+x);
  for(let y=y0;y<y1;y++)if(y<y0+(y1-y0)*.25||y>=y1-(y1-y0)*.25) {
    let count=0;for(let x=x0;x<x1;x++)if(ink.has(y*width+x))count++;
    if(count>(x1-x0)*.85)for(let x=x0;x<x1;x++)ink.delete(y*width+x);
  }
  let enclosure=null;
  while(ink.size) {
    const queue=[ink.values().next().value];let left=width,right=0,top=height,bottom=0,count=0,overlap=0;
    while(queue.length) {
      const p=queue.pop();if(!ink.delete(p))continue;
      const x=p%width,y=Math.floor(p/width);count++;
      left=Math.min(left,x);right=Math.max(right,x);top=Math.min(top,y);bottom=Math.max(bottom,y);
      if(x>=best.x&&x<best.x+best.width&&y>=best.y&&y<best.y+best.height)overlap++;
      for(let dy=-1;dy<=1;dy++)for(let dx=-1;dx<=1;dx++)if(ink.has((y+dy)*width+x+dx))queue.push((y+dy)*width+x+dx);
    }
    const w=right-left+1,h=bottom-top+1;
    if(count>=20&&overlap>=count*.12&&w<=80&&h>=8&&(!enclosure||count>enclosure.count))enclosure={left,right,top,bottom,count};
  }
  if(enclosure)return {x:enclosure.left-2,y:enclosure.top-2,width:enclosure.right-enclosure.left+5,height:enclosure.bottom-enclosure.top+5,confidence:best.confidence};
  return best;
}
