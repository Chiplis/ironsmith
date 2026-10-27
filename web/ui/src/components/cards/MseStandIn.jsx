import {useLayoutEffect,useRef} from 'react';
import {ManaCostIcons,SymbolText} from '@/lib/mana-symbols';
import {cardArtCropUrl} from '@/lib/card-image-variants';

// While a card's own frame renders, show the blank frame for its colors with
// the card's art and text laid into the frame's measured boxes. It is a
// preview: the rendered canvas replaces it with a crossfade.
const POINTS_TO_PIXELS=96/72;
const faceOf=(card,faceIndex)=>{
  const faces=card?.card_faces;
  return Array.isArray(faces)&&faces.length&&!['split','aftermath','adventure','prepare','room','flip'].includes(card.layout)?{...card,...(faces[faceIndex]??faces[0])}:card;
};

// Shrink the rules text until it fits its box, as MSE scales text down.
function FittedText({children,style}){
  const box=useRef(null);
  useLayoutEffect(()=>{
    const node=box.current;
    if(!node)return;
    let low=0.4,high=1;
    node.style.setProperty('--fit',1);
    if(node.scrollHeight<=node.clientHeight)return;
    for(let i=0;i<7;i++){
      const middle=(low+high)/2;node.style.setProperty('--fit',middle);
      if(node.scrollHeight<=node.clientHeight)low=middle;else high=middle;
    }
    node.style.setProperty('--fit',low);
  });
  return <div ref={box} className="mse-stand-in__text" style={style}>{children}</div>;
}

export default function MseStandIn({frame,card,imageUrl,faceIndex=0,placeholder,leaving,onShown,onLeft}){
  const face=faceOf(card,faceIndex),{fields={}}=frame;
  const box=(field,extra={})=>{
    const rect=fields[field];
    if(!rect)return null;
    const font=rect.face??{family:'Georgia, serif',weight:field==='name'||field==='type'||field==='pt'?'bold':'normal',style:'normal'};
    return {left:`${rect.left/frame.width*100}%`,top:`${rect.top/frame.height*100}%`,
      width:`${rect.width/frame.width*100}%`,height:`${rect.height/frame.height*100}%`,
      transform:rect.angle?`rotate(${-rect.angle}deg)`:undefined,
      fontFamily:font.family,fontWeight:font.weight,fontStyle:font.style,color:rect.color??'#000',
      fontSize:`calc(${(rect.size||10)*POINTS_TO_PIXELS/frame.width*100}cqw * var(--fit, 1))`,...extra};
  };
  // The blank frame's cost box fits an empty cost; grow it leftward instead.
  const costBox=()=>{
    const style=box('casting_cost'),rect=fields.casting_cost;
    return {...style,left:'auto',width:'auto',right:`${(1-(rect.left+rect.width)/frame.width)*100}%`};
  };
  const pt=face.power_toughness??(face.power!=null&&face.toughness!=null?`${face.power}/${face.toughness}`:face.loyalty??face.defense);
  const art=imageUrl?cardArtCropUrl(imageUrl):null;
  return <div className="mse-stand-in" data-mse-placeholder={placeholder} data-leaving={leaving?'true':undefined} aria-hidden="true"
    onTransitionEnd={event=>{if(leaving&&event.target===event.currentTarget)onLeft?.();}}>
    <img className="mse-stand-in__frame" src={frame.url} alt="" draggable={false} onLoad={onShown}/>
    {art&&fields.image&&<img className="mse-stand-in__art" src={art} alt="" draggable={false} style={box('image')}
      onLoad={event=>{event.currentTarget.dataset.loaded='true';}}/>}
    {face.name&&fields.name&&<div className="mse-stand-in__line" style={box('name')}>{face.name}</div>}
    {face.mana_cost&&fields.casting_cost&&<div className="mse-stand-in__cost" style={costBox()}>
      <ManaCostIcons cost={face.mana_cost} size={12}/></div>}
    {(face.type_line_display??face.type_line)&&fields.type&&<div className="mse-stand-in__line" style={box('type')}>{face.type_line_display??face.type_line}</div>}
    {face.oracle_text&&fields.text&&<FittedText style={box('text')}>
      {String(face.oracle_text).split('\n').map((line,index)=><p key={index}><SymbolText text={line} symbolSize={12} keywordHelpers={false}/></p>)}
    </FittedText>}
    {pt!=null&&pt!==''&&fields.pt&&<div className="mse-stand-in__pt" style={box('pt')}>{pt}</div>}
  </div>;
}
