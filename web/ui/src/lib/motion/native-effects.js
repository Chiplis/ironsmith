import { eases } from 'animejs/easings';
import { animateCompositor } from './compositor.js';

// A deliberately small finite-effect vocabulary. Compile choreography once
// into real transform/opacity/filter tracks, never animated CSS variables.
const elementStates = new WeakMap();
const aliases = { x: 'translateX', y: 'translateY', rotateZ: 'rotate' };
const transforms = ['translateX', 'translateY', 'translateZ', 'rotate', 'rotateX', 'rotateY', 'scale', 'scaleX', 'scaleY'];
const metadata = new Set(['duration', 'delay', 'ease', 'onComplete', 'autoplay', 'keyframes', 'loop']);
const feedback = /^--card-(jolt|return|flash)-/;
const numberPattern = /-?\d*\.?\d+(?:e[-+]?\d+)?/gi;

function easing(value = 'out(2)') {
  if (typeof value === 'function') return value;
  if (value?.ease) return value.ease;
  const match = /^(\w+)\((.*)\)$/.exec(value);
  if (match && eases[match[1]]) return eases[match[1]](...match[2].split(',').map(Number));
  if (eases[value]) return eases[value];
  throw new Error(`Unsupported native effect easing: ${value}`);
}
function identity(value) {
  return String(value).replace(/(\w+)\(([^)]*)\)/g, (_, name, args) => `${name}(${args.replace(numberPattern, () => /brightness|saturate|contrast|opacity/.test(name) ? '1' : '0')})`);
}
function mix(a, b, t) {
  if (typeof a === 'number' && typeof b === 'number') return a + (b-a)*t;
  if (a === 'none') a = identity(b);
  if (b === 'none') b = identity(a);
  // Filters may introduce blur midway through a ghost fade. Pad missing
  // functions with their neutral values instead of jumping at the endpoint.
  if (String(a).includes('(') && String(b).includes('(')) {
    const functions=value=>new Map([...String(value).matchAll(/([\w-]+)\(([^)]*)\)/g)].map(match=>[match[1],match[0]]));
    const from=functions(a),to=functions(b);
    const names=[...new Set([...to.keys(),...from.keys()])];
    a=names.map(name=>from.get(name)??identity(to.get(name))).join(' ');
    b=names.map(name=>to.get(name)??identity(from.get(name))).join(' ');
  }
  const valuesA = String(a).match(numberPattern)?.map(Number) || [];
  const valuesB = String(b).match(numberPattern)?.map(Number) || [];
  if (valuesA.length !== valuesB.length) return t < 1 ? a : b;
  let i=0;
  return String(b).replace(numberPattern, () => { const n=i++; return valuesA[n]+(valuesB[n]-valuesA[n])*t; });
}
function defaultValue(key, computed) {
  if (transforms.includes(key)) return key.startsWith('scale') ? 1 : 0;
  if (key.startsWith('--')) {
    const raw = computed.getPropertyValue(key).trim();
    return raw || (/scale|brightness|saturate/.test(key) ? 1 : 0);
  }
  return computed[key] || (key === 'opacity' ? 1 : 'none');
}
const normalize = frame => Object.fromEntries(Object.entries(frame).map(([key,value]) => [aliases[key] || key,value]));
function unit(value, suffix) { return typeof value === 'number' || /^-?[\d.]+$/.test(String(value)) ? `${value}${suffix}` : value; }
function propertyFrames(node, params, previous) {
  const computed = getComputedStyle(node);
  const frames = params.keyframes?.map(normalize);
  const normalized = normalize(params);
  const keys = [...new Set((frames || [normalized]).flatMap(frame => Object.keys(frame).filter(key => !metadata.has(key))))];
  for (const key of keys) {
    if (!transforms.includes(key) && !['opacity','filter'].includes(key) && !feedback.test(key)) {
      throw new Error(`Unsupported native effect property: ${key}`);
    }
  }
  const initial = Object.fromEntries(keys.map(key => [key, previous?.values[key] ?? defaultValue(key,computed)]));
  const tracks = Object.fromEntries(keys.map(key => [key,[]]));
  let duration=0;
  if (frames) {
    let state={...initial};
    for (const frame of frames) {
      const length=frame.duration ?? (params.duration ?? 1000)/frames.length;
      for (const key of keys) {
        const next=frame[key] ?? state[key];
        if (length === 0) initial[key]=next;
        else tracks[key].push({ start:duration, end:duration+length, from:state[key], to:next, ease:easing(frame.ease ?? params.ease) });
        state[key]=next;
      }
      duration+=length;
    }
  } else {
    duration=params.duration ?? params.ease?.duration ?? 1000;
    for (const key of keys) {
      const values=Array.isArray(normalized[key]) ? normalized[key] : [initial[key],normalized[key]];
      initial[key]=values[0];
      for (let i=1;i<values.length;i++) tracks[key].push({ start:duration*(i-1)/(values.length-1), end:duration*i/(values.length-1), from:values[i-1], to:values[i], ease:easing(params.ease) });
    }
  }
  const at = time => Object.fromEntries(keys.map(key => {
    const track=tracks[key];
    const step=track.find(step => time <= step.end) || track.at(-1);
    return [key,step ? mix(step.from,step.to,step.ease(Math.max(0,Math.min(1,(time-step.start)/(step.end-step.start))))) : initial[key]];
  }));
  const transformKeys=keys.filter(key => transforms.includes(key));
  const hasFeedback=keys.some(key => feedback.test(key));
  // CSS positioning transforms (e.g. label centering) remain the base. A
  // timeline remembers that base so consecutive steps don't multiply it.
  const base=previous?.base ?? (computed.transform === 'none' ? '' : computed.transform);
  const render = state => {
    const result={};
    if (transformKeys.length) {
      const values={...previous?.values,...state};
      const parts=transforms.filter(key => key in values).map(key => `${key}(${unit(values[key],key.startsWith('translate')?'px':key.startsWith('rotate')?'deg':'')})`);
      result.transform=`${base} ${parts.join(' ')}`.trim();
    }
    for (const key of ['opacity','filter']) if (key in state) result[key]=String(state[key]);
    if (hasFeedback) {
      const get=(suffix,fallback=0) => {
        const key=keys.find(key => key.endsWith(suffix));
        return key ? state[key] : fallback;
      };
      if (keys.some(key => /-(x|y)$/.test(key))) result.translate=`${unit(get('-x'),'px')} ${unit(get('-y'),'px')}`;
      if (keys.some(key => key.endsWith('-scale'))) result.scale=String(get('-scale',1));
      if (keys.some(key => key.endsWith('-rotate'))) result.rotate=unit(get('-rotate'),'deg');
      if (keys.some(key => key.includes('-flash-'))) result.filter=`brightness(${get('-brightness',1)}) saturate(${get('-saturate',1)})`;
    }
    return result;
  };
  // Include every segment boundary, and sample easing at most 8 ms apart.
  // There is no JS interpolation or style write during playback.
  const times=new Set([0,duration]);
  for (const track of Object.values(tracks)) for (const step of track) {
    times.add(step.start); times.add(step.end);
    const count=Math.max(1,Math.ceil((step.end-step.start)/8));
    for (let i=1;i<count;i++) times.add(step.start+(step.end-step.start)*i/count);
  }
  const keyframes=[...times].sort((a,b)=>a-b).map(time=>({ ...render(at(time)), offset:duration ? time/duration : 1 }));
  if (hasFeedback) for (const key of keys) if (feedback.test(key)) node.style.removeProperty(key);
  return { keyframes, duration, values:{...previous?.values,...at(duration)}, base, hasFeedback, at:time=>({...previous?.values,...at(time)}) };
}

export function animateNative(node, params, previous, fill = 'both') {
  const cached=elementStates.get(node);
  const remembered=previous ?? (cached?.inlineTransform === node.style.transform ? cached : undefined);
  const compiled=propertyFrames(node,params,remembered);
  if (compiled.hasFeedback) {
    // Translation precedes the live CSS transform; rotation/scale append to
    // it. This matches the card's original variable composition without
    // freezing its hand-layout/tap transform or animating custom properties.
    const groups=[];
    const first=compiled.keyframes[0];
    if ('translate' in first) groups.push({composite:'add',frames:compiled.keyframes.map(({translate,offset})=>({translate,offset}))});
    if ('rotate' in first || 'scale' in first) groups.push({composite:'add',frames:compiled.keyframes.map(frame=>({
      offset:frame.offset,transform:`rotate(${frame.rotate ?? '0deg'}) scale(${frame.scale ?? 1})`,
    }))});
    if ('opacity' in first || 'filter' in first) groups.push({composite:'replace',frames:compiled.keyframes.map(frame=>Object.fromEntries(Object.entries(frame).filter(([key])=>['opacity','filter','offset'].includes(key))))});
    const motions=groups.map((group,index)=>animateCompositor(node,group.frames,{
      duration:compiled.duration,delay:params.delay??0,fill,commit:false,composite:group.composite,
      onComplete:index===groups.length-1 ? params.onComplete : undefined,
    }));
    const motion={
      cancel:()=>motions.forEach(motion=>motion.cancel()),
      pause:()=>motions.forEach(motion=>motion.pause()),
      finish:()=>motions.forEach(motion=>motion.finish()),
      get currentTime(){return motions[0]?.currentTime;},
      set currentTime(time){motions.forEach(motion=>{motion.currentTime=time;});},
    };
    return {motion,...compiled};
  }
  const motion=animateCompositor(node,compiled.keyframes,{
    duration:compiled.duration, delay:params.delay ?? 0, commit:!compiled.hasFeedback, fill,
    iterations:params.loop === true ? Infinity : 1,
    onComplete:params.onComplete,
  });
  // Preserve interrupted poses just as the previous JS animation did. The
  // cached base prevents repeated transitions from multiplying transforms.
  const cancel=motion.cancel.bind(motion);
  motion.cancel=()=>{
    const elapsed=Number(motion.currentTime)-(params.delay??0);
    if (!compiled.hasFeedback && motion.currentTime != null && (fill === 'both' || elapsed >= 0)) {
      try { if (node.isConnected) motion.commitStyles(); }
      catch (error) { if (error.name !== 'InvalidStateError') throw error; }
      elementStates.set(node,{base:compiled.base,values:Object.fromEntries(Object.entries(compiled.at(Math.max(0,Math.min(compiled.duration,elapsed)))).filter(([key])=>transforms.includes(key))),inlineTransform:node.style.transform});
    }
    cancel();
  };
  return { motion, ...compiled };
}
export function animateEffect(node,params) {
  if (Array.isArray(node)) {
    const motions=node.map((element,index) => animateEffect(element,{...params,delay:typeof params.delay === 'function' ? params.delay(element,index,node.length) : params.delay}));
    return {cancel:()=>motions.forEach(motion=>motion.cancel())};
  }
  return animateNative(node,params).motion;
}

// Finite timelines only: synchronous .add calls compile together, avoiding
// timers for each step. Numeric overlaps are supported for independent tracks.
export function createEffectTimeline({ onComplete } = {}) {
  const entries=[];
  const motions=[];
  let cancelled=false;
  const timeline={
    add(node,params,position) { entries.push({node,params,position}); return timeline; },
    cancel() { cancelled=true; for (const motion of motions) motion.cancel(); },
  };
  queueMicrotask(() => {
    if (cancelled) return;
    let cursor=0, pending=entries.length;
    const states=new Map();
    for (const {node,params,position} of entries) {
      const start=position ?? cursor;
      const compiled=animateNative(node,{...params,delay:start+(params.delay??0),onComplete:()=>{
        params.onComplete?.();
        if (--pending===0 && !cancelled) onComplete?.();
      }},states.get(node), start > 0 ? 'forwards' : 'both');
      states.set(node,compiled);
      motions.push(compiled.motion);
      cursor=Math.max(cursor,start+(params.delay??0)+compiled.duration);
    }
  });
  return timeline;
}
