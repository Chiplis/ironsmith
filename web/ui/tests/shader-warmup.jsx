import React from 'react';
import { createRoot } from 'react-dom/client';
import ZoneMoveEffects from '../src/components/overlays/ZoneMoveEffects.jsx';
import '../src/styles/effects-and-decisions.css';
window.effectsRoot = createRoot(document.getElementById('effects'));
window.effectsRoot.render(<ZoneMoveEffects />);
