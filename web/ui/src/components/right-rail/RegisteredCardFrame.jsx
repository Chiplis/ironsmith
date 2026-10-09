import useUiText from "@/i18n/useUiText";
import {useEffect,useLayoutEffect,useMemo,useRef,useState} from 'react';
import {SymbolText} from '@/lib/mana-symbols';
import {mergeRegisteredLineSegments,registeredColumns,registeredFieldLayouts,registeredRuleAssignments,registeredLinePitch,trimRegisteredNameCosts} from '@/lib/card-region-layout';
import {profileSectionInk} from '@/lib/card-printing-profile';
import {registeredPrintedText} from '@/lib/registered-printed-text';
import {maskRegisteredFrame,orientedRegisteredScan} from '@/lib/card-region-mask';
import CardFrameRulesBox from './CardFrameRulesBox';
import GroupedManaAbility from './GroupedManaAbility';
import {useI18n} from '@/i18n/I18nContext';
import {loadTranslatedCardView} from '@/i18n/cardTranslations';
import '@/styles/card-typography.css';
import './registered-card-frame.css';

const position=b=>({left:`${b.x*100}%`,top:`${b.y*100}%`,width:`${b.width*100}%`,height:`${b.height*100}%`});
const same=(a,b)=>String(a||'').normalize('NFKC').replace(/\s+/g,' ').trim()===String(b||'').normalize('NFKC').replace(/\s+/g,' ').trim();
const flows=field=>['rule','flavor'].includes(field.kind)&&!field.noFlow;
const needsReplacement=(field,text,forceReplace)=>forceReplace || !same(text,field.printedText??field.text) || field.unprinted || field.errata;

function RegisteredRuleText({text,field}) {
  const abilityWord=field.kind==='rule'?text.match(/^([A-Za-z][A-Za-z ’'’-]+) — /):null;
  if(!abilityWord&&!/\[[+−-]?\d+\]/.test(text))return <SymbolText text={text} className="interactive-card-frame__rule-line"/>;
  const body=abilityWord?text.slice(abilityWord[0].length):text;
  return <span className="interactive-card-frame__rule-line">
    {abilityWord&&<><SymbolText text={abilityWord[1]} style={{fontStyle:'italic'}}/><SymbolText text=" — "/></>}
    {body.split(/(\[[+−-]?\d+\])/).filter(Boolean).map((part,index)=>/^\[[+−-]?\d+\]$/.test(part)
      ?<svg key={index} className="registered-card-frame__inline-loyalty" viewBox="0 0 34 22" role="img" aria-label={part.slice(1,-1)}><path d="M1 1H33L30 17L17 21L4 17Z" fill="white" stroke="black" strokeWidth="2"/><text x="17" y="15" textAnchor="middle" fill="black" fontSize="16">{part.slice(1,-1)}</text></svg>
      :<SymbolText key={index} text={part}/>)}</span>;
}

function RegisteredField({field,index,layout,flow,unit,scale=1,onFit,onMeasure,forceReplace,text,actions,group,maskReady,ink,outlined,typography,name,onActivate,highlighted,columnTop=0}) {
  const ui = useUiText();
  // Errata'd printings keep stale wording in the box: replace it even when the
  // live text already equals the current oracle text. A flowed column masks
  // every paragraph, since moved text would otherwise land on printed lines.
  const changed=needsReplacement(field,text,forceReplace);
  const showReplacement=changed&&(field.unprinted||maskReady);
  const action=actions.find(a=>!a.payment_pending&&a.mana_payment_available!==false)||actions[0];
  const clickable=Boolean(action&&onActivate);
  const available=clickable&&!action.payment_pending&&action.mana_payment_available!==false;
  const labelParts=field.labelParts&&text.match(/^(.*?):\s*(Level [23])$/);
  const stacked=field.stackedStats&&text.includes('/');
  const content=stacked?<span className="registered-card-frame__stacked-stats">{text.split('/').map((part,i)=><SymbolText key={i} text={part} className="interactive-card-frame__rule-line" />)}</span>:labelParts?<span className="registered-card-frame__label-parts"><SymbolText text={labelParts[1]+':'} className="interactive-card-frame__rule-line"/><SymbolText text={labelParts[2]} className="interactive-card-frame__rule-line"/></span>:<RegisteredRuleText text={text} field={field}/>;
  const activate=event=>{event.stopPropagation();if(clickable)onActivate(action);};
  // Pixel sizes, not container units: Chromium resolves a var() fallback that
  // carries cq units lazily, so the fitter would measure text at a stale size.
  const quarter=field.rotation===90;
  const logicalUnit=quarter?unit*680/488:unit;
  let placement=layout.bounds;
  if(quarter){const b=layout.bounds,w=b.width*680/488,h=b.height*488/680;placement={x:1-b.y-b.height/2-w/2,y:b.x+b.width/2-h/2,width:w,height:h};}
  const style={...position(placement),'--registered-field-font-size':logicalUnit?`${layout.size*logicalUnit*scale}px`:`${layout.size*scale*100}cqw`,'--registered-field-line-height':layout.lineHeight};
  if(field.fontFamily)for(const section of ['rules','title','type','stats'])style[`--card-${section}-font`]=field.fontFamily;
  if(field.italic)style.fontStyle='italic';
  if(field.rotation===180)style.transform='rotate(180deg)';
  if(quarter)style.transform='rotate(90deg)';
  if(flow&&showReplacement) {
    // Flowed paragraphs size themselves to their text; the column decides where
    // each one starts and how far the last may run before the type shrinks.
    style.top=`${(flow.top-columnTop)*100}%`;
    style.height='auto';
    style.maxHeight=`${Math.max(flow.limit-flow.top,flow.footprint)*100}%`;
  } else if(flow) style.top=`${(flow.top-columnTop)*100}%`;
  if(columnTop) {
    const height=unit*(field.scanAspect||680/488);
    style.top=`${((flow?.top??layout.bounds.y)-columnTop)*height}px`;
    if(style.height!=='auto')style.height=`${layout.bounds.height*height}px`;
    delete style.maxHeight;
  }
  if(showReplacement&&ink)style['--registered-field-ink']=ink;
  if(showReplacement&&profileSectionInk(typography.profile,field.kind)==='light')
    style['--registered-field-shadow']='.035em .035em .025em rgb(0,0,0)';
  return <>
    <div className="registered-card-frame__field" style={style} data-field-kind={field.kind} data-registration-index={index}
      data-fixed-rail={field.labelParts?'true':undefined} data-replaced={showReplacement?'true':'false'} data-live-text={text} data-printed-text={field.text} data-outlined={(field.outlined??outlined)?'true':undefined}
      data-source-text={(field.printedText||registeredPrintedText(field)).replace(/^[+−-]?\d+:\s*/,field.loyaltyCost?'':'$&')}
      data-stack-highlighted={highlighted?'true':undefined} data-unprinted={field.unprinted?'true':undefined}
      data-centred={layout.centred?'true':undefined} data-flow-top={flow?flow.top.toFixed(4):undefined} data-flow-bottom={flow?flow.bottom.toFixed(4):undefined} data-flow-limit={flow?flow.limit.toFixed(4):undefined}>
      {showReplacement ? <CardFrameRulesBox measurementAxis={quarter?"reverse-x":field.rotation===180?"reverse-y":field.scanAspect<1?"x":"y"} label={ui(text)} refitKey={`${unit}|${scale}`} onFit={onFit?fit=>onFit(fit*scale):undefined} onMeasure={onMeasure}>
        {group?<GroupedManaAbility group={group} name={name} onActivate={onActivate}/>:actions.length?
          <button className="registered-card-frame__action" data-available={available?'true':'false'} disabled={!clickable} onClick={activate} aria-label={ui("{0}: {1}", { 0: name, 1: text })}>{content}</button>:content}
      </CardFrameRulesBox>:group?<div className="registered-card-frame__mana-hotspots">
        {group.options.map((option,index)=>{
          const selected=option.actions.find(a=>!a.payment_pending&&a.mana_payment_available!==false)||option.actions[0];
          return <button key={option.output} disabled={!selected||!onActivate} aria-label={ui("Activate {0}: {1}{2}{3}", { 0: name, 1: group.prefix, 2: option.output, 3: group.suffix })}
            style={{left:`${45+index*50/group.options.length}%`,width:`${50/group.options.length}%`}}
            onPointerDown={event=>event.stopPropagation()} onClick={event=>{event.stopPropagation();if(selected&&onActivate)onActivate(selected);}} />;
        })}
      </div>:actions.length?<button className="registered-card-frame__hotspot" data-available={available?'true':'false'} disabled={!clickable} aria-label={ui("{0}: {1}", { 0: name, 1: text })}
        onPointerDown={event=>event.stopPropagation()} onClick={activate}><span className="sr-only">{text}</span></button>:<span className="sr-only">{text}</span>}
    </div>
  </>;
}

// Replacement type is sized from the printed lines in the face that renders it.
function fieldMeasurer(typography) {
  const ctx=document.createElement('canvas').getContext('2d');
  return (kind,field)=>{
    const family=field?.fontFamily||(['name','subtitle'].includes(kind)?typography.title:kind==='type'?typography.type:['stats','tier-stats','loyalty-cost'].includes(kind)?typography.stats:typography.rules);
    const weight=['name','subtitle'].includes(kind)?typography.titleWeight:kind==='type'?typography.style['--card-type-weight']:['stats','tier-stats','loyalty-cost'].includes(kind)?typography.style['--card-stats-weight']:400;
    return (text,italic=false)=>{
      ctx.font=`${italic||kind==='flavor'||kind==='subtitle'?'italic ':''}${weight} 100px ${family}`;
      const metrics=ctx.measureText(text);
      return {width:metrics.width,height:metrics.actualBoundingBoxAscent+metrics.actualBoundingBoxDescent,content:metrics.fontBoundingBoxAscent+metrics.fontBoundingBoxDescent};
    };
  };
}

export default function RegisteredCardFrame({registration,imageUrl:sourceImageUrl,typography,rulesView,name,typeLine,stats,flavorText,onActivate,highlighted,interactive=true}) {
  const [oriented,setOriented]=useState(null);
  useEffect(()=>{
    if(!registration.rotation)return;
    let active=true;
    orientedRegisteredScan(sourceImageUrl,registration.rotation).then(url=>{if(active)setOriented({source:sourceImageUrl,rotation:registration.rotation,url});});
    return ()=>{active=false;};
  },[sourceImageUrl,registration.rotation]);
  const imageUrl=registration.rotation?(oriented?.source===sourceImageUrl&&oriented.rotation===registration.rotation?oriented.url:null):sourceImageUrl;
  const fields=useMemo(()=>registration.fields.flatMap(field=>field.rotation===90?[field]:mergeRegisteredLineSegments(trimRegisteredNameCosts([field],fieldMeasurer(typography)('name')))),[registration,typography]);
  const assignments=useMemo(()=>registeredRuleAssignments(fields,rulesView),[fields,rulesView]);
  const {locale}=useI18n();
  const [translated,setTranslated]=useState(null);
  useEffect(()=>{
    let active=true;
    const names=fields.filter(f=>f.kind==='name');
    if(names.length<2||locale==='en')return;
    Promise.all(names.map(async field=>[field.face,await loadTranslatedCardView(locale,{
      name:field.text,typeLine:fields.find(f=>f.kind==='type'&&f.face===field.face)?.text,
      rulesText:fields.filter(f=>f.kind==='rule'&&f.face===field.face).map(f=>f.text).join('\n'),
    })])).then(entries=>{if(active)setTranslated({registration,locale,faces:new Map(entries)});});
    return ()=>{active=false;};
  },[fields,registration,locale]);
  const translatedFaces=translated?.registration===registration&&translated.locale===locale?translated.faces:null;
  const layouts=useMemo(()=>{
    const natural=registeredFieldLayouts(fields.map(f=>f.orientedGeometry?{...f,...f.orientedGeometry}:f),fieldMeasurer(typography));
    return natural.map((layout,index)=>{
      if(!layout||!flows(fields[index])||fields[index].noFlow)return layout;
      const sizes=fields.flatMap((field,i)=>field.face===fields[index].face&&field.kind==='rule'&&!field.noFlow&&natural[i]?[natural[i].size]:[]).sort((a,b)=>a-b);
      const median=sizes.length?sizes[Math.floor((sizes.length-1)/2)]:layout.size;
      const size=sizes.length?Math.max(sizes[0],median*.75):layout.size;
      const pitch=registeredLinePitch(fields[index]);
      const leading=pitch?pitch*(fields[index].scanAspect||680/488)/size:null;
      return {...layout,size,lineHeight:leading>=.7&&leading<=1.6?leading:layout.lineHeight,span:layout.span*size/layout.size};
    });
  },[fields,typography]);
  const surfaceRef=useRef(null);
  const [unit,setUnit]=useState(0);
  // Replacement paragraphs retain their natural typography while the column scrolls.
  const [sourceScale,setSourceScale]=useState({registration,scale:1});
  const sharedScale=sourceScale.registration===registration?sourceScale.scale:1;
  useLayoutEffect(()=>{
    const node=surfaceRef.current;
    if(!node)return undefined;
    const update=()=>setUnit(node.clientWidth);
    update();
    const observer=new ResizeObserver(update);
    observer.observe(node);
    return ()=>observer.disconnect();
  },[]);
  // What each field shows: translated or live text, and the actions behind it.
  const entries=useMemo(()=>fields.map((field,index)=>{
    if(!field.bounds)return null;
    let text=locale===registration.lang?(field.printedText||field.text):field.text,actions=[],group=null,isHighlighted=false;
    const face=translatedFaces?.get(field.face);
    if(face) {
      const localized=field.kind==='name'?face.name:field.kind==='type'?face.typeLine:field.kind==='rule'?face.rulesText?.replace(/\\n/g,'\n').split('\n')[field.index]:null;
      if(localized&&!same(localized,field.text))text=localized;
    }
    if(field.kind==='rule') {
      const indices=assignments.get(index)||[];
      if(indices.length) {
        const live=indices.map(i=>rulesView.lines[i]).join('\n');
        if(!same(live,field.text))text=live;
        actions=interactive?indices.flatMap(i=>rulesView.actions.get(i)||[]):[];
        group=interactive&&indices.length===1?rulesView.manaGroups.get(indices[0]):null;
        isHighlighted=indices.some(i=>highlighted.has(i));
      }
    } else if(field.kind==='name'&&name&&!name.includes(' // ')&&fields.filter(f=>f.kind==='name').length===1&&!same(name,field.text))text=name;
    else if(field.kind==='type'&&typeLine&&fields.filter(f=>f.kind==='type').length===1&&(locale==='en'||!same(typeLine,field.text)))text=typeLine;
    else if(field.kind==='stats'&&stats&&fields.filter(f=>f.kind==='stats').length===1)text=stats.replace(/\s/g,'');
    else if(field.kind==='flavor'&&flavorText&&fields.filter(f=>f.kind==='flavor').length===1)text=flavorText;
    if(field.kind==='rule'&&field.loyaltyCost)text=text.replace(/^[+−-]?\d+:\s*/, '');
    if(field.kind==='rule'&&field.prototypeRail)text=text.replace(/^Prototype\s+(?:\{[^}]+\})+\s+—\s+[0-9*]+\/[0-9*]+\s*/,'Prototype ');
    if(field.kind==='loyalty-cost'){
      const ability=fields.findIndex(f=>f.kind==='rule'&&f.face===field.face&&f.index===field.ruleIndex);
      const live=rulesView.lines[assignments.get(ability)?.[0]];
      const cost=live?.match(/^([+−-]?\d+):/);if(cost)text=cost[1];
    }
    return {text,actions,group,isHighlighted};
  }),[fields,registration.lang,locale,translatedFaces,assignments,rulesView,highlighted,name,typeLine,stats,flavorText,interactive]);
  const texts=useMemo(()=>entries.map(entry=>entry?.text??''),[entries]);
  // Natural text heights, as fields report them after each fit. A report only
  // counts while the field still shows the text, width and scale it measured.
  const measured=useRef(new Map());
  const [measureVersion,setMeasureVersion]=useState(0);
  const reportMeasure=(index,px,report)=>{
    const previous=measured.current.get(index);
    if(previous&&Math.abs(previous.px-px)<.5&&previous.unit===report.unit&&previous.scale===report.scale&&previous.text===report.text)return;
    measured.current.set(index,{px,...report});
    // Publish each changed measurement. Cancelling a shared animation-frame
    // callback while the fields refit can leave the column using old heights.
    setMeasureVersion(v=>v+1);
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- measureVersion invalidates the measurement ref
  const columns=useMemo(()=>registeredColumns(fields,layouts,texts,measured.current,{unit,scale:sharedScale}),[fields,layouts,texts,unit,sharedScale,measureVersion]);
  useEffect(()=>{
    const original=fields.every((field,index)=>!flows(field)||[field.printedText,registeredPrintedText(field)].some(source=>source&&same(texts[index],source.replace(/^[+−-]?\d+:\s*/,field.loyaltyCost?'':'$&'))));
    if(!original){if(sharedScale!==1)setSourceScale({registration,scale:1});return;}
    const next=Math.max(.65,sharedScale*(columns?.shrink??1));
    if(next<sharedScale-.001)setSourceScale({registration,scale:next});
  },[registration,fields,texts,columns,sharedScale]);
  const maskRegions=useMemo(()=>fields.flatMap((field,index)=>{
    const entry=entries[index];
    if(!entry||field.unprinted||!needsReplacement(field,entry.text,flows(field)&&Boolean(columns?.forced.has(field.face))))return [];
    return [{index,field,family:field.fontFamily||typography[['name','subtitle'].includes(field.kind)?'title':field.kind==='flavor'?'rules':['tier-stats','loyalty-cost'].includes(field.kind)?'stats':field.kind]||typography.rules,profile:typography.profile}];
  }),[fields,entries,columns,typography]);
  // Measurements can recreate the region list without changing the printed
  // pixels to remove. Use a content key so fitting never resets a ready mask.
  const maskKey=JSON.stringify([imageUrl,maskRegions]);
  const [preparedMask,setPreparedMask]=useState(null);
  useEffect(()=>{
    const [url,regions]=JSON.parse(maskKey);
    if(!url||!regions.length)return;
    let active=true;
    maskRegisteredFrame(url,regions).then(value=>{
      if(active)setPreparedMask({key:maskKey,value});
    }).catch(()=>{if(active)setPreparedMask(null);});
    return ()=>{active=false;};
  },[maskKey]);
  const readyMask=preparedMask?.key===maskKey?preparedMask.value:null;
  const inks=new Map(maskRegions.map((region,i)=>[region.index,readyMask?.inks[i]]));
  const outlines=new Map(maskRegions.map((region,i)=>[region.index,readyMask?.outlines[i]]));
  return <article className="registered-card-frame" data-card-era={typography.era} aria-label={name} data-registration-id={registration.id} data-rules-scale={sharedScale} data-rules-shrink={columns?columns.shrink.toFixed(3):undefined}>
    <div className="registered-card-frame__surface" ref={surfaceRef} style={registration.rotation?{width:`${(680/488)*100}%`,aspectRatio:"680/488",maxHeight:"none",transform:`rotate(${-registration.rotation}deg)`}:undefined}>
      <img className="registered-card-frame__scan" src={readyMask?.image||imageUrl||sourceImageUrl} data-mask-ready={readyMask||(!maskRegions.length&&imageUrl)?'true':'false'} alt={name} referrerPolicy="no-referrer" />
      <span className="registered-card-frame__corner-fill registered-card-frame__corner-fill--tl" aria-hidden="true" />
      <span className="registered-card-frame__corner-fill registered-card-frame__corner-fill--tr" aria-hidden="true" />
      <span className="registered-card-frame__corner-fill registered-card-frame__corner-fill--bl" aria-hidden="true" />
      <span className="registered-card-frame__corner-fill registered-card-frame__corner-fill--br" aria-hidden="true" />
      {fields.map((field,index)=>{
        if(columns?.forced.has(field.face)&&flows(field))return null;
        return renderField(field,index);
      })}
      {[...(columns?.forced||[])].map(face=>{
        const indices=fields.map((field,index)=>index).filter(index=>fields[index].face===face&&flows(fields[index])&&columns.positions.has(index));
        if(!indices.length)return null;
        const top=Math.min(...indices.map(index=>columns.positions.get(index).top));
        const limit=columns.positions.get(indices[0]).limit;
        return <div key={face} className="registered-card-frame__column" style={{top:`${top*100}%`,height:`${(limit-top)*100}%`}}>
          <div className="registered-card-frame__column-content">
            {indices.map(index=>renderField(fields[index],index,top))}
          </div>
        </div>;
      })}
    </div>
  </article>;
  function renderField(field,index,columnTop=0) {
        const entry=entries[index];
        if(!entry)return null;
        const shares=flows(field);
        const flow=shares?columns?.positions.get(index)||null:null;
        return <RegisteredField key={index} index={index} field={field} layout={layouts[index]} flow={flow} unit={unit} scale={shares?sharedScale:1}
          columnTop={columnTop} onMeasure={shares?px=>reportMeasure(index,px,{unit,scale:sharedScale,text:entry.text}):undefined}
          forceReplace={shares&&Boolean(columns?.forced.has(field.face))} text={entry.text} actions={entry.actions} group={entry.group} maskReady={Boolean(readyMask)&&field.lines.length>0} ink={inks.get(index)} outlined={outlines.get(index)}
          typography={typography} name={name} onActivate={onActivate} highlighted={entry.isHighlighted}/>;
  }
}
