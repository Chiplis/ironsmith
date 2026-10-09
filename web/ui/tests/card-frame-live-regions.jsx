import {KeywordHelpersProvider} from '../src/lib/mana-symbols';
import {fidelityText} from './card-frame-fidelity-text';
import {createRoot} from 'react-dom/client';
import RegisteredCardFrame from '../src/components/right-rail/RegisteredCardFrame';
import {cardTypography} from '../src/lib/card-typography';
import {I18nProvider} from '../src/i18n/I18nContext';
import '../src/index.css';
const {registration:sourceRegistration,printing,liveRules,liveStats,fidelity=false}=window.__regionFixture;
// Fidelity captures repaint the original content, so a scan-only render cannot
// pass while hiding defects in our replacement typography or cleanup.
const registration=fidelity?{...sourceRegistration,fields:sourceRegistration.fields.map(f=>({...f,printedText:fidelityText(f),errata:!f.opaqueHeader&&!f.opaqueLettering&&!["level-marker","preview-stats"].includes(f.kind)}))}:sourceRegistration;
const face=printing.card_faces?.[registration.face??0]||printing;
const rules=registration.fields.filter(f=>f.kind==='rule');
const typography=cardTypography(printing);
await Promise.all(['title','type','rules','stats'].map(k=>document.fonts.load(`${k==='rules'?400:typography.titleWeight} 100px ${typography[k]}`)));
await document.fonts.load(`italic 400 100px ${typography.rules}`);
const rulesView={lines:fidelity?rules.map(f=>f.printedText):liveRules||rules.map((_,i)=>`Texto dinámico ${i+1}.`),sourceLines:rules.map(f=>[f.text]),
  actions:new Map(rules.map((_,i)=>[i,[{id:i+1}]])),manaGroups:new Map()};
createRoot(document.getElementById('root')).render(<I18nProvider><KeywordHelpersProvider enabled={!fidelity}><div data-live-frame style={{position:'relative',width:420,height:586,...typography.style}}>
  <RegisteredCardFrame registration={registration} imageUrl={registration.source} typography={typography}
    rulesView={rulesView} interactive={!fidelity} highlighted={new Set()} name={fidelity?(registration.fields.find(f=>f.kind==="name")?.printedText||face.name):"Nombre dinámico"} typeLine={fidelity?(registration.fields.find(f=>f.kind==="type")?.printedText||face.type_line):"Tipo dinámico"} stats={liveStats||(fidelity?(face.loyalty??(face.power!=null?`${face.power}/${face.toughness}`:null)):"7/7")} onActivate={action=>window.__activated=action.id}/>
</div></KeywordHelpersProvider></I18nProvider>);
