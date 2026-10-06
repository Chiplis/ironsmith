import { useLayoutEffect, useRef } from 'react';
import { Hand } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useGame } from '@/context/GameContext';
import useUiText from '@/i18n/useUiText';

export default function PriorityHoldButton({ className = '', disabled = false }) {
  const ui = useUiText();
  const { postActionPriorityWindow: window, holdPostActionPriority } = useGame();
  const buttonRef = useRef(null);
  useLayoutEffect(() => {
    if (!window || !buttonRef.current) return;
    const currentTime = performance.now();
    const button = buttonRef.current;
    button.style.setProperty('--hold-delay', `${Math.min(0, (window.startedAt ?? currentTime) - currentTime)}ms`);
    const fitLabel = () => {
      button.style.removeProperty('--hold-label-size');
      const text = button.querySelector('.priority-hold-window-label > span');
      if (!text) return;
      const style = getComputedStyle(button);
      const available = button.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight) - 23;
      const baseSize = parseFloat(style.fontSize);
      const ratio = Math.min(1, Math.max(0.55, available / Math.max(1, text.scrollWidth)));
      button.style.setProperty('--hold-label-size', `${baseSize * ratio}px`);
    };
    fitLabel();
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(fitLabel);
    observer?.observe(button);
    return () => observer?.disconnect();
  }, [window]);
  if (!window) return null;
  const label = <><Hand size={16} aria-hidden="true" /><span>{ui('Hold Priority')}</span></>;
  return (
    <Button
      ref={buttonRef}
      key={window.id}
      type="button"
      className={`decision-main-button priority-hold-window ${className}`}
      disabled={disabled}
      aria-label={ui('Hold Priority')}
      title={ui('Keep priority instead of passing automatically')}
      onClick={holdPostActionPriority}
      style={{ '--hold-duration': `${window.duration}ms`, animationPlayState: window.startedAt === null ? 'paused' : 'running' }}
    >
      <span className="priority-hold-window-label">{label}</span>
      <span className="priority-hold-window-fill" aria-hidden="true">{label}</span>
    </Button>
  );
}
