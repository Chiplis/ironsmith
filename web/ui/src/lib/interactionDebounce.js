const DEFAULT_DEBOUNCE_MS = 100;

const pointerActivationByTarget = new WeakMap();
const clickActivationByTarget = new WeakMap();

function eventTimestamp(event) {
  const ts = Number(event?.timeStamp);
  return Number.isFinite(ts) ? ts : performance.now();
}

function shouldSuppress(map, target, at, debounceMs) {
  if (!target || (typeof target !== "object" && typeof target !== "function")) {
    return false;
  }

  const lastAt = map.get(target) ?? -Infinity;
  if ((at - lastAt) < debounceMs) {
    return true;
  }

  map.set(target, at);
  return false;
}

function suppressEvent(event) {
  if (event?.cancelable) {
    event.preventDefault();
  }
  event?.stopPropagation?.();
}

function isPrimaryPointerDown(event) {
  if (!event) return false;
  if (event.button != null && event.button !== 0) return false;
  return true;
}

function isPointerLikeClick(event) {
  if (!event) return false;
  if (event.detail === 0) return false;
  if (event.button != null && event.button !== 0) return false;
  return true;
}

export function debouncePointerDown(handler, debounceMs = DEFAULT_DEBOUNCE_MS) {
  if (typeof handler !== "function") return handler;

  return (event) => {
    if (!isPrimaryPointerDown(event)) {
      handler(event);
      return;
    }

    const target = event?.currentTarget;
    const at = eventTimestamp(event);
    if (shouldSuppress(pointerActivationByTarget, target, at, debounceMs)) {
      suppressEvent(event);
      return;
    }

    handler(event);
  };
}

export function debounceClick(handler, debounceMs = DEFAULT_DEBOUNCE_MS) {
  if (typeof handler !== "function") return handler;

  return (event) => {
    if (!isPointerLikeClick(event)) {
      handler(event);
      return;
    }

    const target = event?.currentTarget;
    const at = eventTimestamp(event);
    if (shouldSuppress(clickActivationByTarget, target, at, debounceMs)) {
      suppressEvent(event);
      return;
    }

    handler(event);
  };
}

export { DEFAULT_DEBOUNCE_MS };
