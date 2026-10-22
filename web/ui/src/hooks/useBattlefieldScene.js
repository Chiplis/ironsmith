import { useLayoutEffect, useMemo } from 'react';
import { createBattlefieldScene } from '../components/board/scene/battlefield-scene.js';
import '../components/board/scene/battlefield-scene.css';

export default function useBattlefieldScene(rootRef, config, enabled) {
  const api = useMemo(() => {
    const registry = new Map();
    return {
      registry, controller: null,
      register(key, element) { registry.set(key, element); this.controller?.sync(); },
      unregister(key, element) { if (registry.get(key) === element) registry.delete(key); this.controller?.sync(); },
      cellAt(...args) { return this.controller?.cellAt(...args) || null; },
    };
  }, []);
  useLayoutEffect(() => {
    if (!enabled) return;
    const root = rootRef.current;
    api.controller = createBattlefieldScene(root, api.registry);
    root.battlefieldScene = api.controller;
    return () => { api.controller.dispose(); api.controller = null; delete root.battlefieldScene; };
  }, [api, enabled, rootRef]);
  useLayoutEffect(() => { api.controller?.configure(config); });
  return api;
}
