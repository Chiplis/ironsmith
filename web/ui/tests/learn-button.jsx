import React from 'react';
import { createRoot } from 'react-dom/client';
import { I18nProvider } from '../src/i18n/I18nContext';
import { LearnButton } from '../src/components/layout/PlayNavigation';
import '../src/index.css';
createRoot(document.getElementById('root')).render(<I18nProvider><LearnButton /></I18nProvider>);
