import {useLayoutEffect, useRef, useState} from 'react';

// Fit the complete translated label and count to the pile, keeping natural
// letter proportions rather than stretching glyphs or distributing letters.
export default function ZonePileLabel({label, count}) {
  const host = useRef(null);
  const text = useRef(null);
  const [fontSize, setFontSize] = useState(16);
  useLayoutEffect(() => {
    const node = host.current;
    const slot = node.parentElement;
    let active = true;
    const fit = () => {
      if (!active) return;
      const pile = slot.querySelector(':scope > .zone-pile');
      const width = pile?.offsetWidth || slot.clientWidth;
      // Measure at a fixed size so successive resizes never accumulate error.
      text.current.style.fontSize = '16px';
      const naturalWidth = text.current.offsetWidth;
      const size = naturalWidth ? width * 16 / naturalWidth : 16;
      text.current.style.fontSize = '';
      setFontSize(size);
      slot.style.paddingTop = `${Math.ceil(size * 1.15) + 4}px`;
    };
    const observer = new ResizeObserver(fit);
    observer.observe(slot.querySelector(':scope > .zone-pile') || slot);
    fit();
    document.fonts.ready.then(fit);
    return () => { active = false; observer.disconnect(); slot.style.removeProperty('padding-top'); };
  }, [label, count]);
  return <span ref={host} className="zone-pile-label" style={{left:0,right:0,width:'100%',height:'auto',fontSize,lineHeight:1.15,textAlign:'left'}}>
    <span ref={text} style={{display:'inline-block',whiteSpace:'nowrap'}}>{label} <strong>{count}</strong></span>
  </span>;
}
