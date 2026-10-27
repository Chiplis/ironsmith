import {useState} from 'react';
import {abilityActionState,abilityActionReason} from '@/lib/ability-action-state';
import useUiText from '@/i18n/useUiText';
import LoyaltyAbilityText from './LoyaltyAbilityText';
import GroupedManaAbility from '../right-rail/GroupedManaAbility';

export default function CardFrameRules({name,rulesView,onActivate,highlighted}) {
  const ui=useUiText();
  const [chosen,setChosen]=useState(null);
  return <div className="card-frame-rules">
    {rulesView.lines.map((line,index)=>{
      const actions=rulesView.actions.get(index)||[];
      const action=actions.find(item=>abilityActionState(item)==='available')||actions[0];
      const clickable=Boolean(action&&onActivate);
      const status=abilityActionState(action);
      const available=clickable&&status==='available';
      const selected=highlighted?.has(index)||chosen===action;
      return <p key={index} className="inspector-ability-section" data-stack-highlighted={highlighted?.has(index)?'true':undefined}>
        {rulesView.manaGroups?.has(index)?<GroupedManaAbility group={rulesView.manaGroups.get(index)} name={name} onActivate={onActivate}/>
          :action?<button type="button" className="inspector-oracle-line-action" data-available={available?'true':'false'} data-ability-state={selected?'selected':status} aria-pressed={selected}
            title={status==='unavailable'||status==='pending'?abilityActionReason(action):'Activate ability'}
            aria-disabled={!clickable||status==='unavailable'?'true':undefined} aria-label={ui('{0}: {1}',{0:name||'Card',1:line})}
            onPointerDown={event=>event.stopPropagation()}
            onClick={event=>{event.preventDefault();event.stopPropagation();if(clickable&&status!=='unavailable'){if(status==='available')setChosen(action);onActivate(action);}}}><LoyaltyAbilityText text={line} keywordHelpers={false}/></button>
          :<LoyaltyAbilityText text={line}/>}
      </p>;
    })}
  </div>;
}
