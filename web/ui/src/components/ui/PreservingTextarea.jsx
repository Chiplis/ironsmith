import { useCallback, useLayoutEffect, useRef } from "react";

// Textareas used for decklists can be backed by asynchronous validation. Keep
// the user's selection until that validation echoes the value they just typed.
// This does not focus the control or poll; it only restores a selection while
// the same textarea is still active.
export default function PreservingTextarea({ value, onChange, ...props }) {
  const textareaRef = useRef(null);
  const pendingSelectionRef = useRef(null);

  const handleChange = useCallback((event) => {
    const target = event.currentTarget;
    pendingSelectionRef.current = {
      value: target.value,
      start: target.selectionStart,
      end: target.selectionEnd,
      direction: target.selectionDirection,
    };
    onChange?.(event);
  }, [onChange]);

  useLayoutEffect(() => {
    const target = textareaRef.current;
    const pending = pendingSelectionRef.current;
    if (!target || !pending) return;
    if (globalThis.document?.activeElement !== target) {
      pendingSelectionRef.current = null;
      return;
    }

    const length = target.value.length;
    const start = Math.min(Math.max(Number(pending.start) || 0, 0), length);
    const end = Math.min(Math.max(Number(pending.end) || 0, start), length);
    if (target.value !== pending.value) {
      target.setSelectionRange(start, end, pending.direction || "none");
      return;
    }
    target.setSelectionRange(start, end, pending.direction || "none");
    pendingSelectionRef.current = null;
  });

  return (
    <textarea
      {...props}
      ref={textareaRef}
      value={value}
      onChange={handleChange}
    />
  );
}
