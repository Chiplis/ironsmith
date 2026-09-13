import useUiText from "@/i18n/useUiText";
import { useLayoutEffect, useRef } from "react";
import { cardFrameFitKey, measureCardFrameLayout } from '@/lib/card-frame-measurement';
import "@/styles/card-frame-text-fit.css";

export default function CardFrameRulesBox({ children, label, onFit, onMeasure, refitKey }) {
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
      const line = box.querySelector(".interactive-card-frame__rule-line");
      if (!line || !box.clientWidth || !box.clientHeight) return;
      const flavor = box.querySelector(".inspector-flavor-text");
      box.style.removeProperty("--card-fitted-rules-font-size");
      box.style.removeProperty("--card-fitted-flavor-font-size");
      box.style.setProperty("--card-rules-fit-scale", "1");
      box.style.setProperty("--card-rules-spacing-scale", "1");
      box.dataset.textOverflow = "false";
      const preferred = parseFloat(getComputedStyle(box).fontSize);
      const flavorPreferred = flavor ? parseFloat(getComputedStyle(flavor).fontSize) : preferred;
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
        for (const node of box.querySelectorAll('.interactive-card-frame__rule-line')) {
          const range = document.createRange();
          range.selectNodeContents(node);
          const rect = range.getBoundingClientRect();
          if (!rect.height) continue;
          top = Math.min(top, rect.top);
          bottom = Math.max(bottom, rect.bottom);
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
        return [...box.querySelectorAll('.interactive-card-frame__rule-line')].every(node => {
          const range = document.createRange();
          range.selectNodeContents(node);
          const text = range.getBoundingClientRect();
          if (!text.height) return true;
          // A space at a wrapped line end hangs past the box by design; it
          // must not count as horizontal overflow.
          const em = parseFloat(getComputedStyle(node).fontSize) || 16;
          const rects = [...range.getClientRects()];
          const right = Math.max(...rects.filter(rect => !(rect.width < em * .45
            && rects.some(other => other !== rect && Math.abs(other.top - rect.top) < 1 && Math.abs(other.right - rect.left) < 1))).map(rect => rect.right));
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
      // Remove UI-only padding/gaps before changing the printing's typography.
      if (!fits()) {
        box.style.setProperty("--card-rules-spacing-scale", "0");
        if (fits()) {
          let low = 0, high = 1;
          for (let i = 0; i < 8; i++) {
            const spacing = (low + high) / 2;
            box.style.setProperty("--card-rules-spacing-scale", String(spacing));
            if (fits()) low = spacing; else high = spacing;
          }
          box.style.setProperty("--card-rules-spacing-scale", String(low));
        } else {
          // Longer translations/live text may need smaller type, but never
          // collapse an entire card to one-pixel lettering to hide overflow.
          let low = .75, high = 1;
          apply(low);
          if (fits()) {
            for (let i = 0; i < 10; i++) {
              const scale = (low + high) / 2;
              apply(scale);
              if (fits()) low = scale; else high = scale;
            }
            apply(low);
          }
          box.dataset.textOverflow = String(!fits());
        }
      }
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
  }, []);

  useLayoutEffect(() => {
    const key = cardFrameFitKey(boxRef.current);
    if (lastFitRef.current?.key === key && Object.is(lastFitRef.current.refitKey, refitKey)) return;
    fitRef.current?.();
    lastFitRef.current = { key, refitKey };
  });

  return <div ref={boxRef} className="interactive-card-frame__rules" data-fit-text="true" aria-label={ui(label)}>{children}</div>;
}
