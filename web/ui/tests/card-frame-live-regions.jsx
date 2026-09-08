import {createRoot} from 'react-dom/client';
import RegisteredCardFrame from '../src/components/right-rail/RegisteredCardFrame';
import {cardTypography} from '../src/lib/card-typography';
import {I18nProvider} from '../src/i18n/I18nContext';
import '../src/index.css';
const {registration,printing}=window.__regionFixture;
const rules=registration.fields.filter(f=>f.kind==='rule');
const typography=cardTypography(printing);
await Promise.all(['title','type','rules','stats'].map(k=>document.fonts.load(`${k==='rules'?400:typography.titleWeight} 100px ${typography[k]}`)));
await document.fonts.load(`italic 400 100px ${typography.rules}`);
const rulesView={lines:rules.map((_,i)=>`Texto dinámico ${i+1}.`),sourceLines:rules.map(f=>[f.text]),
  actions:new Map(rules.map((_,i)=>[i,[{id:i+1}]])),manaGroups:new Map()};
createRoot(document.getElementById('root')).render(<I18nProvider><div data-live-frame style={{position:'relative',width:420,height:586,...typography.style}}>
  <RegisteredCardFrame registration={registration} imageUrl={registration.source} typography={typography}
    rulesView={rulesView} name="Nombre dinámico" typeLine="Tipo dinámico" stats="7/7" onActivate={action=>window.__activated=action.id}/>
</div></I18nProvider>);
