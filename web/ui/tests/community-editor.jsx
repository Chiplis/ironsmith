/* eslint-disable react-refresh/only-export-components -- Browser fixture. */
import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { I18nProvider } from '../src/i18n/I18nContext';
import CommunityFormatEditor from '../src/components/layout/CommunityFormatEditor';
import { emptyCommunityFormat } from '../src/lib/community-format';
import '../src/index.css';
function Fixture() {
  const [format, setFormat] = useState(emptyCommunityFormat);
  return <main className="fantasy-sheet p-6" style={{ maxWidth: 800, margin: 'auto' }}><h1>Community</h1>
    <CommunityFormatEditor value={format} onChange={value => { setFormat(value); window.__format = value; }} onBusyChange={() => {}} />
  </main>;
}
createRoot(document.getElementById('root')).render(<I18nProvider><Fixture /></I18nProvider>);
