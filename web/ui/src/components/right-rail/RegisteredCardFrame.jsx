import {useEffect,useMemo,useState} from 'react';
import {SymbolText} from '@/lib/mana-symbols';
import {registeredFieldLayouts,registeredRuleAssignments} from '@/lib/card-region-layout';
import {maskRegisteredRegion} from '@/lib/card-region-mask';
import CardFrameRulesBox from './CardFrameRulesBox';
import GroupedManaAbility from './GroupedManaAbility';
import './registered-card-frame.css';

const position=b=>({left:`${b.x*100}%`,top:`${b.y*100}%`,width:`${b.width*100}%`,height:`${b.height*100}%`});
const same=(a,b)=>String(a||'').normalize('NFKC').replace(/\s+/g,' ').trim()===String(b||'').normalize('NFKC').replace(/\s+/g,' ').trim();
function RegisteredField({field,layout,text,actions,group,imageUrl,typography,name,onActivate,highlighted}) {
  // Errata'd printings keep stale wording in the box: replace it even when the
  // live text already equals the current oracle text.
  const changed=!same(text,field.text) || field.unprinted || field.errata;
  const [patch,setPatch]=useState(null);
  useEffect(()=>{
    if(!changed || field.unprinted || !field.lines.length)return;
    let active=true;
    maskRegisteredRegion(imageUrl,field,typography[field.kind==='name'?'title':field.kind==='flavor'?'rules':field.kind]||typography.rules)
      .then(value=>{if(active)setPatch({field,imageUrl,value});});
    return ()=>{active=false;};
  },[changed,field,imageUrl,typography]);
  const ready=field.unprinted || (patch?.field===field && patch.imageUrl===imageUrl);
  const showReplacement=changed&&ready;
  const action=actions.find(a=>!a.payment_pending&&a.mana_payment_available!==false);
  const available=Boolean(action&&onActivate);
  const content=<SymbolText text={text} className="interactive-card-frame__rule-line" />;
  const activate=event=>{event.stopPropagation();if(available)onActivate(action);};
  const style={...position(layout.bounds),'--registered-field-font-size':`${layout.size*100}cqw`,'--registered-field-line-height':layout.lineHeight};
  if(showReplacement&&patch?.value?.ink)style['--registered-field-ink']=patch.value.ink;
  return <>
    {showReplacement&&patch?.field===field&&<img className="registered-card-frame__patch" src={patch.value.image} alt="" style={position(patch.value.bounds)} />}
    <div className="registered-card-frame__field" style={style} data-field-kind={field.kind}
      data-replaced={showReplacement?'true':'false'} data-live-text={text} data-printed-text={field.text}
      data-stack-highlighted={highlighted?'true':undefined} data-unprinted={field.unprinted?'true':undefined}>
      {showReplacement ? <CardFrameRulesBox label={text}>
        {group?<GroupedManaAbility group={group} name={name} onActivate={onActivate}/>:actions.length?
          <button className="registered-card-frame__action" disabled={!available} onClick={activate} aria-label={`${name}: ${text}`}>{content}</button>:content}
      </CardFrameRulesBox>:group?<div className="registered-card-frame__mana-hotspots">
        {group.options.map((option,index)=>{
          const selected=option.actions.find(a=>!a.payment_pending&&a.mana_payment_available!==false);
          return <button key={option.output} disabled={!selected||!onActivate} aria-label={`Activate ${name}: ${group.prefix}${option.output}${group.suffix}`}
            style={{left:`${45+index*50/group.options.length}%`,width:`${50/group.options.length}%`}}
            onPointerDown={event=>event.stopPropagation()} onClick={event=>{event.stopPropagation();if(selected&&onActivate)onActivate(selected);}} />;
        })}
      </div>:actions.length?<button className="registered-card-frame__hotspot" disabled={!available} aria-label={`${name}: ${text}`}
        onPointerDown={event=>event.stopPropagation()} onClick={activate}><span className="sr-only">{text}</span></button>:<span className="sr-only">{text}</span>}
    </div>
  </>;
}

// Replacement type is sized from the printed lines in the face that renders it.
function fieldMeasurer(typography) {
  const ctx=document.createElement('canvas').getContext('2d');
  return kind=>{
    const family=kind==='name'?typography.title:kind==='type'?typography.type:kind==='stats'?typography.stats:typography.rules;
    const weight=kind==='name'?typography.titleWeight:kind==='type'?typography.style['--card-type-weight']:kind==='stats'?typography.style['--card-stats-weight']:400;
    return (text,italic=false)=>{
      ctx.font=`${italic||kind==='flavor'?'italic ':''}${weight} 100px ${family}`;
      const metrics=ctx.measureText(text);
      return {width:metrics.width,height:metrics.actualBoundingBoxAscent+metrics.actualBoundingBoxDescent,content:metrics.fontBoundingBoxAscent+metrics.fontBoundingBoxDescent};
    };
  };
}

export default function RegisteredCardFrame({registration,imageUrl,typography,rulesView,name,typeLine,stats,flavorText,onActivate,highlighted}) {
  const assignments=useMemo(()=>registeredRuleAssignments(registration.fields,rulesView),[registration,rulesView]);
  const fields=registration.fields;
  const layouts=useMemo(()=>registeredFieldLayouts(fields,fieldMeasurer(typography)),[fields,typography]);
  return <article className="registered-card-frame" aria-label={name} data-registration-id={registration.id}>
    <div className="registered-card-frame__surface">
      <img className="registered-card-frame__scan" src={imageUrl} alt={name} referrerPolicy="no-referrer" />
      {fields.map((field,index)=>{
        if(!field.bounds)return null;
        let text=field.text,actions=[],group=null,isHighlighted=false;
        if(field.kind==='rule') {
          const indices=assignments.get(index)||[];
          if(indices.length) {
            text=indices.map(i=>rulesView.lines[i]).join('\n');
            actions=indices.flatMap(i=>rulesView.actions.get(i)||[]);
            group=indices.length===1?rulesView.manaGroups.get(indices[0]):null;
            isHighlighted=indices.some(i=>highlighted.has(i));
          }
        } else if(field.kind==='name'&&name&&!name.includes(' // ')&&fields.filter(f=>f.kind==='name').length===1)text=name;
        else if(field.kind==='type'&&typeLine&&fields.filter(f=>f.kind==='type').length===1)text=typeLine;
        else if(field.kind==='stats'&&stats&&fields.filter(f=>f.kind==='stats').length===1)text=stats.replace(/\s/g,'');
        else if(field.kind==='flavor'&&flavorText&&fields.filter(f=>f.kind==='flavor').length===1)text=flavorText;
        return <RegisteredField key={index} field={field} layout={layouts[index]} text={text} actions={actions} group={group} imageUrl={imageUrl}
          typography={typography} name={name} onActivate={onActivate} highlighted={isHighlighted}/>;
      })}
    </div>
  </article>;
}
