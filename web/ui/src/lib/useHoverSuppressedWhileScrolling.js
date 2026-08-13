import { useCallback, useEffect, useRef, useState } from "react";

const DEFAULT_IDLE_MS = 140;

export function useHoverSuppressedWhileScrolling({ onScrollStart, idleMs = DEFAULT_IDLE_MS } = {}) {
  const nodeRef = useRef(null);
  const attachedNodeRef = useRef(null);
  const timeoutRef = useRef(null);
  const idleMsRef = useRef(idleMs);
  const onScrollStartRef = useRef(onScrollStart);
  const hoverSuppressedRef = useRef(false);
  const [hoverSuppressed, setHoverSuppressed] = useState(false);

  useEffect(() => {
    onScrollStartRef.current = onScrollStart;
  }, [onScrollStart]);

  useEffect(() => {
    idleMsRef.current = idleMs;
  }, [idleMs]);

  const markScrollingActive = useCallback(() => {
    if (!hoverSuppressedRef.current) {
      hoverSuppressedRef.current = true;
      setHoverSuppressed(true);
      onScrollStartRef.current?.();
    }

    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }
    timeoutRef.current = window.setTimeout(() => {
      timeoutRef.current = null;
      hoverSuppressedRef.current = false;
      setHoverSuppressed(false);
    }, idleMsRef.current);
  }, []);

  const detachListeners = useCallback((node) => {
    if (!node) return;
    node.removeEventListener("scroll", markScrollingActive);
    node.removeEventListener("wheel", markScrollingActive);
    node.removeEventListener("touchmove", markScrollingActive);
  }, [markScrollingActive]);

  const attachScrollableRef = useCallback((node) => {
    nodeRef.current = node;
  }, []);

  useEffect(() => {
    const nextNode = nodeRef.current;
    const attachedNode = attachedNodeRef.current;

    if (attachedNode === nextNode) return;

    detachListeners(attachedNode);
    attachedNodeRef.current = null;

    if (!nextNode) return;

    nextNode.addEventListener("scroll", markScrollingActive, { passive: true });
    nextNode.addEventListener("wheel", markScrollingActive, { passive: true });
    nextNode.addEventListener("touchmove", markScrollingActive, { passive: true });
    attachedNodeRef.current = nextNode;
  });

  useEffect(
    () => () => {
      detachListeners(attachedNodeRef.current);
      attachedNodeRef.current = null;
      nodeRef.current = null;
      hoverSuppressedRef.current = false;
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
    },
    [detachListeners]
  );

  return {
    attachScrollableRef,
    hoverSuppressed,
  };
}
