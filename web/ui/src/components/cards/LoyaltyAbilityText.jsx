import {SymbolText} from '@/lib/mana-symbols';
import {splitLoyaltyAbility} from '@/lib/loyalty-ability';

export default function LoyaltyAbilityText({text,keywordHelpers}) {
  const loyalty=splitLoyaltyAbility(text);
  if (!loyalty) return <SymbolText text={text} keywordHelpers={keywordHelpers}/>;
  const path=loyalty.direction==='up'?'M3 12 L24 3 L45 12 L42 35 L6 35 Z'
    :loyalty.direction==='down'?'M3 5 L45 5 L42 28 L24 37 L6 28 Z'
    :'M3 6 L45 6 L42 34 L6 34 Z';
  return <span className="scrollable-loyalty-ability">
    <svg className="scrollable-loyalty-badge" data-loyalty-cost={loyalty.cost} viewBox="0 0 48 40" aria-hidden="true">
      <path d={path} fill="#171719" stroke="#c9c9d1" strokeWidth="3"/>
      <text x="24" y="27" textAnchor="middle" fill="#fff" fontFamily="Georgia,serif" fontSize={loyalty.cost.length>2?'19':'24'} fontWeight="bold">{loyalty.cost}</text>
    </svg>
    <span><span className="sr-only">{loyalty.cost}: </span><SymbolText text={loyalty.text} keywordHelpers={keywordHelpers}/></span>
  </span>;
}
