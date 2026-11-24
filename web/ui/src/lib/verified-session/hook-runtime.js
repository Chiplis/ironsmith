// The four React hooks the peer-lobby stack uses (useState, useRef, useCallback,
// useEffect), with a second implementation that runs without React.
//
// Inside a React render the calls go to React unchanged. Inside a headless
// render (`createHeadlessHook`) they use the slot tables below, which follow the
// same rules: state setters are stable and re-render on change, `useCallback`
// keeps its function while the dependency list is unchanged, and effects run
// after the render that changed their dependencies, cleanup first. This lets the
// whole `usePeerLobby` stack run in node and inside hosts that are not React
// apps (Manabrew's adapter, test harnesses) with no change to its code.

import * as React from "react";

let activeInstance = null;

function depsChanged(previous, next) {
  if (!previous || !next) return true;
  if (previous.length !== next.length) return true;
  for (let index = 0; index < next.length; index += 1) {
    if (!Object.is(previous[index], next[index])) return true;
  }
  return false;
}

export function useState(initial) {
  return activeInstance ? activeInstance.useState(initial) : React.useState(initial);
}

export function useRef(initial) {
  return activeInstance ? activeInstance.useRef(initial) : React.useRef(initial);
}

export function useCallback(callback, deps) {
  return activeInstance ? activeInstance.useCallback(callback, deps) : React.useCallback(callback, deps);
}

export function useEffect(effect, deps) {
  return activeInstance ? activeInstance.useEffect(effect, deps) : React.useEffect(effect, deps);
}

const MAX_SETTLE_TURNS = 200;

// Runs `render(inputs)` (a hook function) as a long-lived component outside
// React. `update(inputs)` supplies new inputs and re-renders, like a parent
// re-rendering with new props. State changes re-render on a microtask.
//
//   const lobby = createHeadlessHook(usePeerLobby, inputs);
//   lobby.current          // latest return value
//   lobby.subscribe(fn)    // called after every committed render
//   lobby.update(inputs)   // new inputs
//   lobby.dispose()        // unmount: run every effect cleanup
export function createHeadlessHook(render, initialInputs, { onError } = {}) {
  const slots = [];
  let cursor = 0;
  let inputs = initialInputs;
  let scheduled = false;
  let rendering = false;
  let disposed = false;
  let pendingEffects = [];
  const listeners = new Set();
  const instance = { current: undefined, error: null };

  const schedule = () => {
    if (scheduled || disposed) return;
    scheduled = true;
    queueMicrotask(() => {
      scheduled = false;
      if (!disposed) run();
    });
  };

  const api = {
    useState(initial) {
      const index = cursor++;
      let slot = slots[index];
      if (!slot) {
        slot = { value: typeof initial === "function" ? initial() : initial };
        slot.set = (next) => {
          const value = typeof next === "function" ? next(slot.value) : next;
          if (Object.is(value, slot.value)) return;
          slot.value = value;
          schedule();
        };
        slots[index] = slot;
      }
      return [slot.value, slot.set];
    },
    useRef(initial) {
      const index = cursor++;
      if (!slots[index]) slots[index] = { ref: { current: initial } };
      return slots[index].ref;
    },
    useCallback(callback, deps) {
      const index = cursor++;
      const slot = slots[index];
      if (slot && !depsChanged(slot.deps, deps)) return slot.callback;
      slots[index] = { callback, deps };
      return callback;
    },
    useEffect(effect, deps) {
      const index = cursor++;
      const slot = slots[index] || (slots[index] = { deps: undefined, cleanup: undefined, ran: false });
      if (!slot.ran || depsChanged(slot.deps, deps)) {
        pendingEffects.push({ slot, effect, deps });
      }
    },
  };

  function commitEffects() {
    const effects = pendingEffects;
    pendingEffects = [];
    for (const { slot } of effects) {
      const cleanup = slot.cleanup;
      slot.cleanup = undefined;
      if (typeof cleanup === "function") cleanup();
    }
    for (const { slot, effect, deps } of effects) {
      slot.ran = true;
      slot.deps = deps;
      const cleanup = effect();
      slot.cleanup = typeof cleanup === "function" ? cleanup : undefined;
    }
  }

  function run() {
    if (rendering || disposed) {
      schedule();
      return;
    }
    rendering = true;
    try {
      cursor = 0;
      const previous = activeInstance;
      activeInstance = api;
      try {
        instance.current = render(inputs);
        instance.error = null;
      } catch (error) {
        instance.error = error;
        if (onError) onError(error);
        else throw error;
      } finally {
        activeInstance = previous;
      }
      commitEffects();
    } finally {
      rendering = false;
    }
    for (const listener of [...listeners]) listener(instance.current);
  }

  run();

  return {
    get current() { return instance.current; },
    get error() { return instance.error; },
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    update(nextInputs) {
      inputs = nextInputs;
      run();
    },
    // Resolves once no render is pending (state settled).
    async settled() {
      for (let turn = 0; turn < MAX_SETTLE_TURNS && (scheduled || rendering); turn += 1) {
        await Promise.resolve();
      }
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      for (const slot of slots) {
        if (slot && typeof slot.cleanup === "function") {
          const cleanup = slot.cleanup;
          slot.cleanup = undefined;
          cleanup();
        }
      }
      listeners.clear();
    },
  };
}
