import React from 'react';
import RegisteredCardFrame from '../src/components/right-rail/RegisteredCardFrame';
import {I18nProvider} from '../src/i18n/I18nContext';
import {cardTypography} from '../src/lib/card-typography';
import {createRoot} from 'react-dom/client';
import CardFrameRulesBox from '../src/components/right-rail/CardFrameRulesBox';
import '../src/index.css';
import '../src/components/right-rail/registered-card-frame.css';
export function Sample({name,text,height,flavor}) {
  return <div data-sample={name} style={{width:360,height,display:'flex','--sampled-rules-font-size':'20px','--printed-flavor-font-size':'18px'}}>
    <CardFrameRulesBox label={name}><div className="interactive-card-frame__rules-body">
      <div className="interactive-card-frame__rule"><span className="interactive-card-frame__rule-line">{text}</span></div>
      {flavor && <div className="inspector-flavor-text interactive-card-frame__rule-line">{flavor}</div>}
    </div></CardFrameRulesBox>
  </div>;
}
// A registered field: no padding, the printed size, one line box tall.
// Several paragraphs in one box, as a permanent that gained abilities has.
export function Paragraphs({name,texts,height}) {
  return <div data-sample={name} style={{width:360,height,display:'flex','--sampled-rules-font-size':'20px'}}>
    <CardFrameRulesBox label={name}><div className="interactive-card-frame__rules-body">
      {texts.map((text,index)=><div key={index} className="interactive-card-frame__rule"><span className="interactive-card-frame__rule-line">{text}</span></div>)}
    </div></CardFrameRulesBox>
  </div>;
}
export function Registered({name,text,size,lineHeight,height}) {
  return <div data-sample={name} className="registered-card-frame__field" data-replaced="true" style={{position:'relative',width:360,height,'--registered-field-font-size':size,'--registered-field-line-height':lineHeight}}>
    <CardFrameRulesBox label={name}><span className="interactive-card-frame__rule-line">{text}</span></CardFrameRulesBox>
  </div>;
}
const typography=cardTypography({frame:'2003'});
const registration={id:'overflow-fixture',lang:'en',fields:[
  {kind:'rule',face:'front',text:'Flying',unprinted:true,lines:[],bounds:{x:.1,y:.6,width:.8,height:.025}},
  {kind:'flavor',face:'front',text:'Above the clouds.',unprinted:true,lines:[],bounds:{x:.1,y:.8,width:.8,height:.025}},
]};
createRoot(document.getElementById('root')).render(<I18nProvider><>
  <div data-column-fixture style={{position:'relative',width:360,height:360*680/488,...typography.style}}>
    <RegisteredCardFrame registration={registration} imageUrl="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='488' height='680'/%3E" typography={typography}
      rulesView={{lines:['Flying. '.repeat(150)],sourceLines:[['Flying']],actions:new Map(),manaGroups:new Map()}}
      name="Overflow fixture" highlighted={new Set()} />
  </div>
  <Registered name="registered" height={30} size="22px" lineHeight={1} text="Protección contra humanos" />
  <div data-sample="grow" className="interactive-card-frame-stage" data-source-frame="true" style={{position:'relative',width:360,height:180,'--printed-rules-font-size':'12px'}}>
    <div className="interactive-card-frame" style={{width:360,height:180,padding:0}}>
      <CardFrameRulesBox label="grow"><div className="interactive-card-frame__rules-body">
        <span className="interactive-card-frame__rule-line">Flying <i className="rules-reminder-text">(This creature can fly.)</i></span>
        <div className="inspector-flavor-text interactive-card-frame__rule-line">Above the clouds.</div>
      </div></CardFrameRulesBox>
    </div>
  </div>
  <Sample name="short" height={160} text="Add green mana." flavor="As patient and generous as life." />
  <Sample name="spacing" height={66} text="Add green mana." flavor="A separate italic line." />
  <style>{`[data-sample="reserved"] .interactive-card-frame__rules {padding-bottom:30px!important;}`}</style>
  <Sample name="reserved" height={110} text={"The final line must leave room for the printed power and toughness plaque. ".repeat(2)} />
  <Sample name="long" height={100} text={'Texto traducido muy largo. '.repeat(60)} />
  <Paragraphs name="paragraphs" height={160} texts={[
    'Protection from Humans',
    'Pay 1 life, Sacrifice another creature: Put a -1/-1 counter on up to one target creature and draw a card.',
    '{B}{B}, Discard a card: Proliferate.',
    '{4}: Put a +1/+1 counter on this creature.',
    'Remove a +1/+1 counter from this creature: It deals 1 damage to any target.',
  ]} />
</></I18nProvider>);
