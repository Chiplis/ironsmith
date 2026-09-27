import React from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { usePeerLobbyConnections } from '../src/hooks/peer-lobby/connections.js';
import { usePeerLobbyAuditMaterial } from '../src/hooks/peer-lobby/audit-material.js';

export function mountOpeningServices(base, services) {
  function Harness() {
    Object.assign(services.current, usePeerLobbyConnections(base, services), usePeerLobbyAuditMaterial(base, services));
    return null;
  }
  const root = createRoot(document.getElementById('root'));
  flushSync(() => root.render(React.createElement(Harness)));
  return root;
}
