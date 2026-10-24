import React, { useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import { useWasmGame } from '../src/hooks/useWasmGame';

export function Fixture() {
  const hook = useWasmGame();
  useEffect(() => { window.auditHook = hook; }, [hook]);
  return null;
}
window.auditRoot = createRoot(document.getElementById('root'));
window.auditRoot.render(<Fixture />);
