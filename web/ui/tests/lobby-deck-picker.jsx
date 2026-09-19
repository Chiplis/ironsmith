/* eslint-disable react-refresh/only-export-components -- Browser fixture. */
import React from 'react';
import { createRoot } from 'react-dom/client';
import { I18nProvider } from '../src/i18n/I18nContext';
import CompetitiveDeckPicker from '../src/components/layout/CompetitiveDeckPicker';
import '../src/index.css';

function Fixture() {
  // The real picker lives inside the lobby sheet, whose chrome styles every
  // control; the fixture reproduces that scope so the flat rules are tested
  // against it.
  return (
    <div data-slot="sheet-content" style={{ padding: 24, width: 720 }}>
      <CompetitiveDeckPicker format="modern" onApply={(applied) => { window.__applied = applied; }} />
    </div>
  );
}

createRoot(document.getElementById('root')).render(<I18nProvider><Fixture /></I18nProvider>);
