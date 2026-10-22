import { useLayoutEffect, useRef } from 'react';
import { useGame } from '@/context/GameContext';

// Only a surface that acquired a pause may release it. Stack cards keep their
// hover across timer changes; the hold button requires fresh mouse movement.
export default function usePriorityHoverPause({ enabled = true, requireMovement = false } = {}) {
  const { postActionPriorityWindow, pausePostActionPriority, resumePostActionPriority } = useGame();
  const id = enabled ? postActionPriorityWindow?.id : undefined;
  const hovered = useRef(false);
  const pausedId = useRef(null);
  const pause = () => {
    if (id == null || pausedId.current === id) return;
    pausedId.current = id;
    pausePostActionPriority(id);
  };
  const release = () => {
    if (pausedId.current == null) return;
    resumePostActionPriority(pausedId.current);
    pausedId.current = null;
  };

  useLayoutEffect(() => {
    if (!requireMovement && hovered.current && id != null) {
      pausedId.current = id;
      pausePostActionPriority(id);
    }
    return () => {
      if (pausedId.current == null) return;
      resumePostActionPriority(pausedId.current);
      pausedId.current = null;
    };
  }, [id, requireMovement, pausePostActionPriority, resumePostActionPriority]);

  return {
    onMouseEnter: () => {
      hovered.current = true;
      if (!requireMovement) pause();
    },
    onMouseMove: pause,
    onMouseLeave: () => {
      hovered.current = false;
      release();
    },
  };
}
