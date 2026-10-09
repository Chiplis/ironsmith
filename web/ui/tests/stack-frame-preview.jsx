import React, { useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { I18nProvider } from '../src/i18n/I18nContext';
import { HoverProvider } from '../src/context/HoverContext';
import { DragProvider } from '../src/context/DragContext';
import FloatingCardPreview from '../src/components/right-rail/FloatingCardPreview';
import '../src/index.css';
const source = { id: 10, stable_id: 7, name: 'Resolving source', type_line: 'Artifact', oracle_text: 'Draw a card.' };
const entry = { id: 20, inspect_object_id: 10, source_stable_id: 7, name: source.name, ability_kind: 'Triggered' };
const initial = { perspective: 0, players: [{ id: 0, battlefield: [source, { id: 20, name: 'Wrong card' }] }], stack_objects: [], decision: {kind: 'priority'} };
const game = { objectDetails: async () => null };
function Fixture() {
 const [state, setState] = useState(initial);
 useEffect(() => { window.setStackState = setState; window.stackFixture = { initial, entry }; }, []);
 return <GameContext.Provider value={{ game, state }}><div data-stack-preview-anchor style={{position:'fixed',left:10,top:20,width:80,height:100}} /><div data-my-zone id="desktop-stack-fixture" style={{display:'none'}}><div className="my-zone-stack-rail" style={{position:'fixed',left:12,top:100,width:300,height:400}}><div className="stack-timeline-entry" style={{height:48}}>Top resolving entry</div><div>Lower entries</div></div></div><FloatingCardPreview /></GameContext.Provider>;
}
createRoot(document.getElementById('root')).render(<I18nProvider><HoverProvider><DragProvider><Fixture /></DragProvider></HoverProvider></I18nProvider>);
