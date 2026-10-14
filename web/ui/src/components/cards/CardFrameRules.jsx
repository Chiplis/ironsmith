import useUiText from '@/i18n/useUiText';
import {SymbolText} from '@/lib/mana-symbols';
import GroupedManaAbility from '../right-rail/GroupedManaAbility';

export default function CardFrameRules({name,rulesView,onActivate,highlighted}) {
  const ui=useUiText();
  return <div className="card-frame-rules">
    {rulesView.lines.map((line,index)=>{
      const actions=rulesView.actions.get(index)||[];
      const action=actions.find(item=>!item.payment_pending&&item.mana_payment_available!==false)||actions[0];
      const clickable=Boolean(action&&onActivate);
      const available=clickable&&!action.payment_pending&&action.mana_payment_available!==false;
      return <p key={index} className="inspector-ability-section" data-stack-highlighted={highlighted?.has(index)?'true':undefined}>
        {rulesView.manaGroups?.has(index)?<GroupedManaAbility group={rulesView.manaGroups.get(index)} name={name} onActivate={onActivate}/>
          :action?<button type="button" className="inspector-oracle-line-action" data-available={available?'true':'false'}
            aria-disabled={clickable?undefined:'true'} aria-label={ui('{0}: {1}',{0:name||'Card',1:line})}
            onPointerDown={event=>event.stopPropagation()}
            onClick={event=>{event.preventDefault();event.stopPropagation();if(clickable)onActivate(action);}}><SymbolText text={line}/></button>
          :<SymbolText text={line}/>}
      </p>;
    })}
  </div>;
}
