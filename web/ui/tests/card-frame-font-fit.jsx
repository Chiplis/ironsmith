import React from 'react';
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
export function Registered({name,text,size,lineHeight,height}) {
  return <div data-sample={name} className="registered-card-frame__field" data-replaced="true" style={{position:'relative',width:360,height,'--registered-field-font-size':size,'--registered-field-line-height':lineHeight}}>
    <CardFrameRulesBox label={name}><span className="interactive-card-frame__rule-line">{text}</span></CardFrameRulesBox>
  </div>;
}
createRoot(document.getElementById('root')).render(<>
  <Registered name="registered" height={30} size="22px" lineHeight={1} text="Protección contra humanos" />
  <Sample name="short" height={160} text="Add green mana." flavor="As patient and generous as life." />
  <Sample name="spacing" height={66} text="Add green mana." flavor="A separate italic line." />
  <style>{`[data-sample="reserved"] .interactive-card-frame__rules {padding-bottom:30px!important;}`}</style>
  <Sample name="reserved" height={110} text={"The final line must leave room for the printed power and toughness plaque. ".repeat(2)} />
  <Sample name="long" height={100} text={'Texto traducido muy largo. '.repeat(60)} />
</>);
