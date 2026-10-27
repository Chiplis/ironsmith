import {useEffect,useRef,useState} from 'react';
import {createRoot} from 'react-dom/client';
import CardFramePresentation from '../src/components/cards/CardFramePresentation';
import {I18nProvider} from '../src/i18n/I18nContext';
import '../src/index.css';
import './ability-affordance-preview.css';
const card={name:'Yawgmoth, Thran Physician',mana_cost:'{2}{B}{B}',colors:['B'],type_line:'Legendary Creature — Human Cleric',power:'2',toughness:'4',set:'mh1',rarity:'mythic',oracle_text:'Protection from Humans\nPay 1 life, Sacrifice another creature: Put a -1/-1 counter on up to one target creature and draw a card.\n{B}{B}, Discard a card: Proliferate.'};
const lines=card.oracle_text.split('\n');
const rulesView={lines,actions:new Map([[1,[{id:1}]],[2,[{id:2}]]])};
function Example({mode,title,description}) {
 const root=useRef(null),[regions,setRegions]=useState([]),[selected,setSelected]=useState(mode==='selected'?1:null),[hovered,setHovered]=useState(mode==='hover'?1:null);
 useEffect(()=>{
  const update=()=>{
   const element=root.current;
   if(!element?.querySelector('[data-card-glint-frame="ready"]'))return;
   const bounds=element.getBoundingClientRect(),groups=new Map();
   for(const button of element.querySelectorAll('.mse-card-hit-target')){
    const label=button.getAttribute('aria-label');if(!label)continue;
    const id=label.includes('Pay 1 life')?1:2,box=button.getBoundingClientRect();
    const group=groups.get(id)||{id,top:box.top,bottom:box.bottom};
    group.top=Math.min(group.top,box.top);group.bottom=Math.max(group.bottom,box.bottom);groups.set(id,group);
   }
   const next=[...groups.values()].map(group=>({id:group.id,top:(group.top-bounds.top-4)/bounds.height*100,height:(group.bottom-group.top+8)/bounds.height*100}));
   setRegions(previous=>JSON.stringify(previous)===JSON.stringify(next)?previous:next);
  };
  const observer=new MutationObserver(update);observer.observe(root.current,{childList:true,subtree:true,attributes:true,attributeFilter:['data-card-glint-frame']});
  window.addEventListener('resize',update);update();return()=>{observer.disconnect();window.removeEventListener('resize',update);};
 },[]);
 return <section className="ability-study"><header><span>{mode==='idle'?'01':mode==='hover'?'02':'03'}</span><h2>{title}</h2></header>
 <div className="study-card" ref={root}><CardFramePresentation card={card} rulesView={rulesView} imageUrl={window.__abilityArt} onActivate={()=>{}}/>
 {regions.map(region=><button key={region.id} className="parchment-action" data-hovered={hovered===region.id} data-selected={selected===region.id} style={{top:`${region.top}%`,height:`${region.height}%`}} aria-label={`Choose ability: ${lines[region.id]}`}
 onMouseEnter={()=>setHovered(region.id)} onMouseLeave={()=>setHovered(mode==='hover'?1:null)} onClick={()=>setSelected(region.id)}><span className="ability-gem">{selected===region.id?'✓':'◆'}</span>{(hovered===region.id||selected===region.id)&&<span className="ability-engraving">{selected===region.id?'CHOSEN':'ACTIVATE'}</span>}</button>)}
 </div>
 <div className="study-under-card">{selected!==null?<><span className="chosen-cost">Next: pay costs & choose targets</span><button onClick={()=>setSelected(null)}>Cancel <kbd>ESC</kbd></button></>:<span className="available-note">◆ 2 abilities available · choose on the card</span>}</div>
 <p>{description}</p></section>;
}
createRoot(document.getElementById('root')).render(<I18nProvider><main className="ability-concept"><div className="concept-intro"><span className="eyebrow">IRONSMITH · INTERACTION STUDY</span><h1>The card is the control.</h1><p>Activate the printed ability. Keep the choice attached to its source.</p><span className="prototype-label">Interactive concept · no game actions</span></div><div className="study-grid"><Example mode="idle" title="Discover" description="A brass edge and small rune mark available abilities. Static text stays plain."/><Example mode="hover" title="Aim" description="The whole ability block lights up. Blank parchment inside the block is clickable too."/><Example mode="selected" title="Choose" description="The chosen ability stays lit while costs and targets are selected. Nothing is paid by hovering."/></div><footer>Double-click a permanent to focus this view. Choose an ability explicitly; keep the existing payment and targeting flow.</footer></main></I18nProvider>);
