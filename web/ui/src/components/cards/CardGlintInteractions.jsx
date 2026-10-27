import AbilityTextInk from './AbilityTextInk';
import {abilityActionState,abilityActionReason,abilityHitRegions} from '@/lib/ability-action-state';
import {useMemo,useState} from 'react';
import {cardGlintRegions} from '@/lib/card-glint/interactions.mjs';
import {mseRulesHelperView} from '@/lib/mse/rules-helper-ranges.mjs';
import {KeywordHelperText,GameSymbolHelper} from '@/lib/mana-symbols';
import {useI18n} from '@/i18n/I18nContext';

export default function CardGlintInteractions({result,regions,rulesView,onActivate,highlighted,name}) {
  const [hovered,setHovered]=useState(null);
  const [chosen,setChosen]=useState(null);
  const hitRegions=useMemo(()=>abilityHitRegions(regions),[regions]);
  const states=useMemo(()=>new Map(hitRegions.map(region=>{
    const key=`${region.lineIndex}:${region.option||''}`;
    const action=region.actions.find(item=>abilityActionState(item)==='available')||region.actions[0];
    const selected=highlighted?.has(region.lineIndex)||chosen===action;
    return [key,selected?'selected':hovered===key?abilityActionState(action):'idle'];
  })),[hitRegions,highlighted,chosen,hovered]);
  const {locale}=useI18n({optional:true});
  const helpers=useMemo(()=>cardGlintRegions(result,mseRulesHelperView(rulesView,locale)),[result,rulesView,locale]);
  const position=region=>({left:`${region.left/488*100}%`,top:`${region.top/680*100}%`,
    width:`${region.width/488*100}%`,height:`${region.height/680*100}%`,transform:`rotate(${region.angle}deg)`});
  return <div className="mse-card-interactions">
    <AbilityTextInk result={result} regions={regions} states={states}/>
    {hitRegions.map((region,index)=>{
      const action=region.actions.find(item=>abilityActionState(item)==='available')||region.actions[0];
      if(!action&&!highlighted?.has(region.lineIndex))return null;
      const status=abilityActionState(action);
      const key=`${region.lineIndex}:${region.option||''}`;
      const props={className:'mse-card-hit-target',style:{left:`${region.left/488*100}%`,top:`${region.top/680*100}%`,
        width:`${region.width/488*100}%`,height:`${region.height/680*100}%`,transform:`rotate(${region.angle}deg)`},
        'data-stack-highlighted':highlighted?.has(region.lineIndex)?'true':undefined};
      return action?<button key={index} {...props} type="button"
        data-ability-state={states.get(key)} aria-pressed={states.get(key)==='selected'}
        title={status==='unavailable'||status==='pending'?abilityActionReason(action):'Activate ability'}
        onFocus={()=>setHovered(key)} onBlur={()=>setHovered(null)}
        data-hovered={hovered===`${region.lineIndex}:${region.option||''}`?'true':undefined}
        onPointerEnter={()=>setHovered(`${region.lineIndex}:${region.option||''}`)} onPointerLeave={()=>setHovered(null)}
        aria-label={`${name}: ${rulesView.lines[region.lineIndex]}${region.option?` (${region.option})`:''}`}
        data-available={status==='available'?'true':'false'}
        aria-disabled={!onActivate||status==='unavailable'?'true':undefined}
        onPointerDown={event=>event.stopPropagation()}
        onClick={event=>{event.preventDefault();event.stopPropagation();if(onActivate&&status!=='unavailable'){if(status==='available')setChosen(action);onActivate(action);}}}/>
        :<span key={index} {...props}/>;
    })}
    {helpers.map((region,index)=>{
      const helper=region.helper,style=position(region);
      if(helper.type==='keyword')return <KeywordHelperText key={`helper-${index}`} text={helper.text} context={helper.context} rule={helper.rule} respectContext
        className="mse-card-rule-helper" style={style}><span className="mse-helper-label">{helper.text}</span></KeywordHelperText>;
      return <GameSymbolHelper key={`helper-${index}`} sym={helper.symbol}>
        <button type="button" className="mse-card-rule-helper mse-card-symbol-helper" style={style} aria-label={`{${helper.symbol}}`}
          onPointerDown={event=>event.stopPropagation()} onClick={event=>event.stopPropagation()}/>
      </GameSymbolHelper>;
    })}
  </div>;
}
