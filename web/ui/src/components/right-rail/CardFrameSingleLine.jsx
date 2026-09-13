import { useLayoutEffect, useRef } from "react";
import { cardFrameFitKey, measureCardFrameLayout } from '@/lib/card-frame-measurement';
import "@/styles/card-frame-text-fit.css";

// Keep the printing's preferred typography unless the complete line won't fit.
export default function CardFrameSingleLine({ as = "span", className, children }) {
  const textRef = useRef(null);
  const fitRef = useRef(null);
  const lastFitKeyRef = useRef(null);

  useLayoutEffect(() => {
    const text = textRef.current;
    const range = document.createRange();
    let frame = 0;
    let active = true;
    const metricsContext=document.createElement('canvas').getContext('2d');
    const alignBaseline=()=>{
      const section = text.classList.contains('interactive-card-frame__title') ? 'title'
        : text.classList.contains('interactive-card-frame__type') ? 'type'
        : text.classList.contains('interactive-card-frame__stats-text') ? 'stats' : null;
      if (!section) return;
      const stage=text.closest('.interactive-card-frame-stage');
      const baseline=Number(stage?.style.getPropertyValue(`--printed-${section}-baseline`));
      text.style.removeProperty('transform');
      if(!baseline)return;
      const frame=stage.querySelector('.interactive-card-frame').getBoundingClientRect();
      const scale=frame.width/Number(stage.style.getPropertyValue('--printed-scan-width'));
      const css=getComputedStyle(text);
      metricsContext.font=`${css.fontWeight} ${css.fontSize} ${css.fontFamily}`;
      const metrics=metricsContext.measureText(text.textContent);
      const ascent=metrics.fontBoundingBoxAscent,descent=metrics.fontBoundingBoxDescent;
      if(!Number.isFinite(ascent)||!Number.isFinite(descent))return;
      const offset=(parseFloat(css.lineHeight)-ascent-descent)/2+ascent;
      const current=text.getBoundingClientRect().top+offset;
      const bounds = JSON.parse(stage.style.getPropertyValue(`--printed-${section}-text-bounds`) || 'null');
      const rect = text.getBoundingClientRect();
      const inkLeft = rect.left - metrics.actualBoundingBoxLeft;
      const dx = !bounds ? 0 : section === 'stats'
        ? frame.left + (bounds.x + bounds.width / 2) * scale - (inkLeft + (metrics.actualBoundingBoxLeft + metrics.actualBoundingBoxRight) / 2)
        : frame.left + bounds.x * scale - inkLeft;
      text.style.transform=`translate(${dx}px, ${frame.top+baseline*scale-current}px)`;
    };
    const fit = () => measureCardFrameLayout(text, () => {
      if (!text.getBoundingClientRect().width) return;

      // Reset before measuring so shorter text, new fonts, and wider cards can
      // recover their original size. Flex layout reserves mana/count space.
      text.style.removeProperty("font-size");
      const preferred = parseFloat(getComputedStyle(text).fontSize);
      range.selectNodeContents(text);
      const fits=()=>range.getBoundingClientRect().width<=text.getBoundingClientRect().width;
      if (fits()) {alignBaseline();return;}

      let low = 0;
      let high = preferred;
      for (let i = 0; i < 12; i++) {
        const size = (low + high) / 2;
        text.style.fontSize = `${size}px`;
        if (fits()) low = size;
        else high = size;
      }
      text.style.fontSize = `${Math.floor(low * 100) / 100}px`;
      alignBaseline();
    });
    const scheduleFit = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => { if (active) fit(); });
    };
    fitRef.current = fit;
    const observer = new ResizeObserver(scheduleFit);
    observer.observe(text);
    document.fonts.ready.then(() => { if (active) scheduleFit(); });
    document.fonts.addEventListener("loadingdone", scheduleFit);
    return () => {
      active = false;
      fitRef.current = null;
      cancelAnimationFrame(frame);
      observer.disconnect();
      document.fonts.removeEventListener("loadingdone", scheduleFit);
    };
  }, []);

  // The parent can update sampled font metrics without changing the text.
  useLayoutEffect(() => {
    const key = cardFrameFitKey(textRef.current);
    if (key === lastFitKeyRef.current) return;
    fitRef.current?.();
    lastFitKeyRef.current = key;
  });

  return as === "h2"
    ? <h2 ref={textRef} className={className}>{children}</h2>
    : <span ref={textRef} className={className}>{children}</span>;
}
