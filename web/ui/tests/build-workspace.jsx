import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { I18nProvider } from '../src/i18n/I18nContext';
import CardCreationControls from '../src/components/layout/CardCreationControls';
import '../src/index.css';
const game = {
  autocompleteCardNames: async query => ['Lightning Bolt','Yavimaya, Cradle of Growth'].filter(name=>name.toLowerCase().includes(query.toLowerCase())),
  previewCustomCard: async draft => { if(draft.faces.some(face=>!face.name || !face.cardTypes.length)) throw new Error('Card requires name and type'); return {canCreate:true,faces:draft.faces.map(face=>({...face,compiledText:[face.oracleText]}))}; },
  createCustomCard: async payload => { window.lastAdded = {custom:payload}; },
  addCardToZone: async (...args) => { window.lastAdded = {catalog:args}; },
};
function App() {
 const [status,setStatus] = useState('');
 return <I18nProvider><GameContext.Provider value={{game,state:{players:[{id:0,name:'Alice'}],perspective:0},multiplayer:{mode:'idle'},runWasmInteraction:fn=>fn(),refresh:async()=>{},setStatus}}><CardCreationControls navigation/><output>{status}</output></GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById('root')).render(<App/>);
