import { cloneElement, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { localCardPreviewBounds } from '@/lib/card-preview-bounds';

// Move one persistent card surface between its slot and the overlay. Keeping
// the portal destination stable preserves card state and gesture listeners.
export default function HandHoverPortal({ enabled, active, objectId, children }) {
  const slotRef = useRef(null);
  const [container] = useState(() => {
    const element = document.createElement('div');
    element.style.display = 'contents';
    return element;
  });
  const [position, setPosition] = useState(null);
  const [navigationScope, setNavigationScope] = useState(null);
  useLayoutEffect(() => {
    const focused = container.contains(document.activeElement) ? document.activeElement : null;
    (active ? document.body : slotRef.current)?.appendChild(container);
    focused?.focus({ preventScroll: true });
  }, [active, container, enabled]);
  useLayoutEffect(() => () => { container.remove(); }, [container]);
  useLayoutEffect(() => {
    if (!active) return undefined;
    const update = () => {
      const rect = slotRef.current?.closest('.hand-layout-item')?.getBoundingClientRect();
      if (!rect) return;
      setNavigationScope(slotRef.current.closest('[data-card-navigation-scope]'));
      const bounds = localCardPreviewBounds();
      const left = Math.max(bounds.margin, Math.min(window.innerWidth - bounds.width - bounds.margin, rect.left + rect.width / 2 - bounds.width / 2));
      setPosition({ left, top: window.innerHeight - bounds.height - bounds.margin, width: bounds.width, height: bounds.height });
    };
    update();
    window.addEventListener('resize', update);
    document.addEventListener('scroll', update, true);
    return () => {
      window.removeEventListener('resize', update);
      document.removeEventListener('scroll', update, true);
    };
  }, [active, objectId]);
  if (!enabled) return children;
  const enlarged = active && position;
  const card = enlarged ? cloneElement(children, {
    getKeyboardNavigationScope: () => navigationScope,
    className: [children.props.className, 'hand-card--portal'].filter(Boolean).join(' '),
    style: { ...children.props.style, width: '100%', minWidth: 0, maxWidth: 'none', height: '100%', minHeight: 0, maxHeight: 'none',
      '--card-scale': 1, '--card-translate-x': '0px', '--card-translate-y': '0px', '--card-rotate': '0deg' },
  }) : children;
  return <>
    <div ref={slotRef} style={{ display: 'contents' }}>
      {active ? <div aria-hidden="true" style={{ ...children.props.style, visibility: 'hidden', pointerEvents: 'none', transformOrigin: '50% 100%',
        transform: 'translate(var(--card-translate-x), var(--card-translate-y)) rotate(var(--card-rotate)) scale(var(--card-scale))' }} /> : null}
    </div>
    {createPortal(
      <div className={enlarged ? 'hand-hover-portal' : undefined} style={enlarged ? position : { display: 'contents' }}>
        {card}
      </div>, container,
    )}
  </>;
}
