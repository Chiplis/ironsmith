import { createRoot } from 'react-dom/client';
import { HoverProvider } from '../src/context/HoverContext';
import { I18nProvider } from '../src/i18n/I18nContext';
import OpponentHandFan from '../src/components/board/OpponentHandFan';
import '../src/index.css';

createRoot(document.getElementById('root')).render(
  <I18nProvider><HoverProvider>
    <div data-my-zone style={{ position: 'absolute', bottom: 0, width: '100%' }}>
      <div className="hand-zone-row" style={{ width: '80%', margin: 'auto' }}>
        <div className="game-card hand-card" style={{ width: 200, height: 280 }} />
      </div>
    </div>
    <div data-opponent-zone="1" style={{ position: 'absolute', top: 100, width: '100%', height: 300 }}>
      <OpponentHandFan player={{ id: 1, name: 'Opponent', hand_size: 7 }} mirrorHand />
      <div className="battlefield-row" id="opponent-field" style={{ position: 'absolute', top: 40, left: '30%', width: '40%', height: 100, padding: 0 }}>
        <div className="battlefield-row-card" style={{ width: '100%', height: '100%' }} />
      </div>
    </div>
    <div data-opponent-zone="2" style={{ position: 'absolute', top: 0, left: 0 }}>
      <div className="battlefield-row"><div className="battlefield-row-card" style={{ width: '100vw', height: 40 }} /></div>
    </div>
  </HoverProvider></I18nProvider>
);
