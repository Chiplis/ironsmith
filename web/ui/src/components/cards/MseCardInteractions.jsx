import {useMemo,useState} from 'react';
import {mseInteractionRegions} from '@/lib/mse/interaction-regions.mjs';
import {mseRulesHelperView} from '@/lib/mse/rules-helper-ranges.mjs';
import {KeywordHelperText,GameSymbolHelper} from '@/lib/mana-symbols';
import {useI18n} from '@/i18n/I18nContext';

export default function MseCardInteractions({result,rulesView,onActivate,highlighted,name}) {
  const [hovered,setHovered]=useState(null);
  const regions=useMemo(()=>mseInteractionRegions(result,rulesView),[result,rulesView]);
  const {locale}=useI18n({optional:true});
  const helpers=useMemo(()=>mseInteractionRegions(result,mseRulesHelperView(rulesView,locale)),[result,rulesView,locale]);
  const position=region=>({left:`${region.left/result.width*100}%`,top:`${region.top/result.height*100}%`,
    width:`${region.width/result.width*100}%`,height:`${region.height/result.height*100}%`,transform:`rotate(${region.angle}deg)`});
  return <div className="mse-card-interactions">
    {regions.map((region,index)=>{
      const action=region.actions.find(item=>!item.payment_pending&&item.mana_payment_available!==false)||region.actions[0];
      if(!action&&!highlighted?.has(region.lineIndex))return null;
      const props={className:'mse-card-hit-target',style:{left:`${region.left/result.width*100}%`,top:`${region.top/result.height*100}%`,
        width:`${region.width/result.width*100}%`,height:`${region.height/result.height*100}%`,transform:`rotate(${region.angle}deg)`},
        'data-stack-highlighted':highlighted?.has(region.lineIndex)?'true':undefined};
      return action?<button key={index} {...props} type="button"
        data-hovered={hovered===`${region.lineIndex}:${region.option||''}`?'true':undefined}
        onPointerEnter={()=>setHovered(`${region.lineIndex}:${region.option||''}`)} onPointerLeave={()=>setHovered(null)}
        aria-label={`${name}: ${rulesView.lines[region.lineIndex]}${region.option?` (${region.option})`:''}`}
        data-available={!action.payment_pending&&action.mana_payment_available!==false?'true':'false'}
        aria-disabled={onActivate?undefined:'true'}
        onPointerDown={event=>event.stopPropagation()}
        onClick={event=>{event.preventDefault();event.stopPropagation();onActivate?.(action);}}/>
        :<span key={index} {...props}/>;
    })}
    {helpers.map((region,index)=>{
      const helper=region.helper,style=position(region);
      if(helper.type==='keyword')return <KeywordHelperText key={`helper-${index}`} text={helper.text} rule={helper.rule} respectContext
        className="mse-card-rule-helper" style={style}><span className="mse-helper-label">{helper.text}</span></KeywordHelperText>;
      return <GameSymbolHelper key={`helper-${index}`} sym={helper.symbol}>
        <button type="button" className="mse-card-rule-helper mse-card-symbol-helper" style={style} aria-label={`{${helper.symbol}}`}
          onPointerDown={event=>event.stopPropagation()} onClick={event=>event.stopPropagation()}/>
      </GameSymbolHelper>;
    })}
  </div>;
}
