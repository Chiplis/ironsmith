import useUiText from "@/i18n/useUiText";
import { useLayoutEffect, useRef } from "react";
import { cardFrameFitKey, measureCardFrameLayout } from '@/lib/card-frame-measurement';
import "@/styles/card-frame-text-fit.css";

export default function CardFrameRulesBox({ children, label, onFit, onMeasure, refitKey, measurementAxis="y" }) {
  const ui = useUiText();
  const boxRef = useRef(null);
  const fitRef = useRef(null);
  const lastFitRef = useRef(null);
  const onFitRef = useRef(onFit);
  const onMeasureRef = useRef(onMeasure);
  useLayoutEffect(() => {
    onFitRef.current = onFit;
    onMeasureRef.current = onMeasure;
  });

  useLayoutEffect(() => {
    const box = boxRef.current;
    let frame = 0;
    let active = true;
    const fit = () => measureCardFrameLayout(box, () => {
      const line = box.querySelector(".interactive-card-frame__rule-line, .inspector-mana-line");
      if (!line || !box.clientWidth || !box.clientHeight) return;
      const flavor = box.querySelector(".inspector-flavor-text");
      box.style.removeProperty("--card-fitted-rules-font-size");
      box.style.removeProperty("--card-fitted-flavor-font-size");
      box.style.setProperty("--card-rules-fit-scale", "1");
      box.style.setProperty("--card-rules-spacing-scale", "1");
      box.dataset.textOverflow = "false";
      const preferred = parseFloat(getComputedStyle(box).fontSize);
      const flavorPreferred = preferred;
      for (const paragraph of box.querySelectorAll('[data-reminder-aligned]')) {
        paragraph.style.removeProperty('margin-top');
        delete paragraph.dataset.reminderAligned;
      }
      let reminderAnchor = null;
      const reminderSource = getComputedStyle(box).getPropertyValue('--printed-reminder-first-line');
      if (reminderSource) {
        const {line: printedLine} = JSON.parse(reminderSource);
        for (const reminder of box.querySelectorAll('.rules-reminder-text')) {
          const start = reminder.textContent.indexOf(printedLine);
          const paragraph = reminder.closest('.interactive-card-frame__rule');
          if (start < 0 || !paragraph) continue;
          const walker = document.createTreeWalker(reminder, NodeFilter.SHOW_TEXT);
          let offset = start;
          while (walker.nextNode()) {
            const node = walker.currentNode;
            if (offset < node.length) {
              const range = document.createRange();
              range.setStart(node, offset); range.setEnd(node, offset + 1);
              reminderAnchor = {paragraph, range};
              paragraph.dataset.reminderAligned = 'true';
              break;
            }
            offset -= node.length;
          }
          if (reminderAnchor) break;
        }
      }
      // The height the text needs at the preferred size, before any fitting:
      // registered frames flow their paragraphs from it instead of shrinking.
      if (onMeasureRef.current) {
        let top = Infinity, bottom = -Infinity;
        for (const node of box.querySelectorAll('.interactive-card-frame__rule-line, .inspector-mana-line')) {
          const range = document.createRange();
          range.selectNodeContents(node);
          const rect = range.getBoundingClientRect();
          if (!rect.height) continue;
          top = Math.min(top, measurementAxis.endsWith("x")?rect.left:rect.top);
          bottom = Math.max(bottom, measurementAxis.endsWith("x")?rect.right:rect.bottom);
        }
        onMeasureRef.current(bottom > top ? bottom - top : 0);
      }
      const apply = scale => {
        box.style.setProperty("--card-fitted-rules-font-size", `${preferred * scale}px`);
        box.style.setProperty("--card-fitted-flavor-font-size", `${flavorPreferred * scale}px`);
        box.style.setProperty("--card-rules-fit-scale", String(scale));
      };
      // Scroll extents include highlights and flex layout, which can report
      // overflow even with ample room for the actual text. Fit glyph contents.
      // Registered fields have no padding: text may run to their edges, while
      // padded boxes keep a small margin so decoration never clips.
      const fits = () => {
        if (reminderAnchor) {
          const {paragraph, range} = reminderAnchor;
          paragraph.style.marginTop = '0px';
          const natural = range.getBoundingClientRect().top - box.getBoundingClientRect().top;
          paragraph.style.marginTop = `calc(max(0px, var(--printed-reminder-offset) - ${natural}px) * var(--card-rules-spacing-scale, 1))`;
        }
        // Match the source's flavor start while allowing longer live rules to
        // push it down. Measure without its spacer, then restore only the room
        // left over; reducing UI spacing can close this gap before shrinking.
        if (flavor && getComputedStyle(box).getPropertyValue('--printed-flavor-offset')) {
          box.style.setProperty('--card-flavor-natural-top', '100000px');
          const style = getComputedStyle(flavor);
          const top = flavor.getBoundingClientRect().top - box.getBoundingClientRect().top
            + (parseFloat(style.paddingTop) || 0) + (parseFloat(style.borderTopWidth) || 0);
          box.style.setProperty('--card-flavor-natural-top', `${top}px`);
        }
        const bounds = box.getBoundingClientRect();
        const style = getComputedStyle(box);
        const stats = box.closest('[data-source-frame="true"]')
          ? box.closest('.interactive-card-frame__rules-section')?.querySelector('.interactive-card-frame__printed-stats')?.getBoundingClientRect()
          : null;
        const inset = side => Math.max(0, parseFloat(style.getPropertyValue(`padding-${side}`)) || 0);
        return [...box.querySelectorAll('.interactive-card-frame__rule-line, .inspector-mana-line')].every(node => {
          const range = document.createRange();
          range.selectNodeContents(node);
          const text = range.getBoundingClientRect();
          if (!text.height) return true;
          const em = parseFloat(getComputedStyle(node).fontSize) || 16;
          const quarterTurn=measurementAxis.endsWith('x');
          const rects=[...range.getClientRects()];
          const glyphRects=rects.filter(rect=>!((quarterTurn?rect.height:rect.width)<em*.45
            &&rects.some(other=>other!==rect&&(quarterTurn
              ?Math.abs(other.left-rect.left)<1&&(Math.abs(other.bottom-rect.top)<1||Math.abs(other.top-rect.bottom)<1)
              :Math.abs(other.top-rect.top)<1&&(Math.abs(other.right-rect.left)<1||Math.abs(other.left-rect.right)<1)))));
          const glyph={left:Math.min(...glyphRects.map(r=>r.left)),right:Math.max(...glyphRects.map(r=>r.right)),
            top:Math.min(...glyphRects.map(r=>r.top)),bottom:Math.max(...glyphRects.map(r=>r.bottom))};
          // In the normalized quarter-turn layout, physical left is the
          // logical top. Like upright text, its font ascent may extend above
          // the line box; fit logical left/right and bottom instead.
          if(measurementAxis==='reverse-y')return glyph.top>=bounds.top-.5
            && glyph.left>=bounds.left-.5&&glyph.right<=bounds.right+.5;
          if(measurementAxis==='reverse-x')return glyph.left>=bounds.left-.5
            && glyph.bottom<=bounds.bottom+.5&&glyph.top>=bounds.top-.5;
          if(measurementAxis==='x')return glyph.right<=bounds.right+.5
            && glyph.bottom<=bounds.bottom+.5&&glyph.top>=bounds.top-.5;
          // A space at a wrapped line end hangs past the box by design; it
          // must not count as horizontal overflow.
          const right = glyph.right;
          // The P/T badge only occupies the lower-right corner. A short final
          // reminder line can use the printed space to its left without
          // forcing the entire paragraph upward to clear a full-width gutter.
          const clearsBottom = text.bottom <= bounds.bottom - inset('bottom') + .5
            || stats && rects.every(rect => rect.bottom <= bounds.bottom - 4 + .5
              && (rect.bottom <= stats.top - 2 || rect.right <= stats.left - 3 || rect.left >= stats.right + 3));
          return clearsBottom
            && right <= bounds.right - inset('right') + .5 && text.left >= bounds.left + inset('left') - .5;
        });
      };
      // Live abilities keep the natural printed size. Short rules may grow to
      // the frame's normal maximum (4.5% of card width); never shrink to hide
      // overflow. All visible rules, reminder and flavor participate in fits().
      const registered = box.closest('.registered-card-frame__field');
      const stage = box.closest('.interactive-card-frame-stage');
      const width = stage?.querySelector('.interactive-card-frame')?.clientWidth || box.clientWidth;
      const maximum = registered ? preferred : Math.max(preferred,
        stage?.dataset.sourceFrame === 'true' ? width * .045 : Math.min(16.5, width * .045));
      if (fits() && maximum > preferred) {
        let low = 1, high = maximum / preferred;
        apply(high);
        if (!fits()) {
          for (let i = 0; i < 12; i++) {
            const scale = (low + high) / 2;
            apply(scale);
            if (fits()) low = scale; else high = scale;
          }
          apply(low);
        }
      }
      // Names, types and stats have fixed rails. Fit changed labels into those
      // rails; rules retain natural size and use their scrolling region.
      const normalize=text=>String(text||'').replace(/\s+/g,' ').trim();
      const unchangedSource=registered?.dataset.sourceText&&normalize(registered.dataset.sourceText)===normalize(registered.dataset.liveText);
      if (registered && (!['rule', 'flavor'].includes(registered.dataset.fieldKind) || registered.dataset.fixedRail==='true'||unchangedSource) && !fits()) {
        let low = .25, high = 1;
        for (let i = 0; i < 12; i++) {
          const scale = (low + high) / 2;
          apply(scale);
          if (fits()) low = scale; else high = scale;
        }
        apply(low);
      }
      box.dataset.textOverflow = String(!fits());
      box.scrollTop = 0;
      onFitRef.current?.(Number(box.style.getPropertyValue("--card-rules-fit-scale")) || 1);
    });
    const scheduleFit = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => { if (active) fit(); });
    };
    fitRef.current = fit;
    const observer = new ResizeObserver(scheduleFit);
    observer.observe(box);
    if (box.firstElementChild) observer.observe(box.firstElementChild);
    document.fonts.ready.then(() => { if (active) scheduleFit(); });
    document.fonts.addEventListener("loadingdone", scheduleFit);
    return () => {
      active = false;
      fitRef.current = null;
      cancelAnimationFrame(frame);
      observer.disconnect();
      document.fonts.removeEventListener("loadingdone", scheduleFit);
    };
  }, [measurementAxis]);

  useLayoutEffect(() => {
    const key = cardFrameFitKey(boxRef.current);
    if (lastFitRef.current?.key === key && Object.is(lastFitRef.current.refitKey, refitKey)) return;
    fitRef.current?.();
    lastFitRef.current = { key, refitKey };
  });

  return <div ref={boxRef} className="interactive-card-frame__rules" data-fit-text="true" aria-label={ui(label)}>{children}</div>;
}
