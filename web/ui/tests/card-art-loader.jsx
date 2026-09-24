/* eslint-disable react-refresh/only-export-components -- Browser fixture. */
import React, { useState, useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { HoverProvider } from '../src/context/HoverContext';
import { DragProvider } from '../src/context/DragContext';
import { ObjectSelectionProvider } from '../src/context/ObjectSelectionContext';
import { I18nProvider } from '../src/i18n/I18nContext';
import GameCard from '../src/components/cards/GameCard';
import LoadingCardArt from '../src/components/cards/LoadingCardArt';
import '../src/index.css';
function Fixture() {
  const [src,setSrc]=useState(() => window.__cardArtLoaderSource || '/tests/art/first.svg');
  const [pending,setPending]=useState(() => Boolean(window.__cardArtLoaderPending));
  useEffect(()=>{
    window.changeArt=(nextSrc) => { setSrc(nextSrc); setPending(false); };
    window.setCardArtMetadataSettled=()=>setPending(false);
    return () => { delete window.changeArt; delete window.setCardArtMetadataSettled; };
  },[]);
  const state={players:[{id:0,name:'Alice',battlefield:[]}],perspective:0,decision:null};
  const value={state,multiplayer:{mode:'idle'},playerAccentOverrides:{},game:null};
  return <GameContext.Provider value={value}><ObjectSelectionProvider><HoverProvider><DragProvider>
    <main style={{display:'flex',gap:40,padding:40}}>
      {(window.__cardArtLoaderPendingFixture ? [] : ['hand','battlefield','mobile-token']).map((mode,index)=><div key={mode} data-probe={mode} style={{position:'relative',width:180,height:252}}>
        <GameCard card={{id:index+1,name:'',controller:0,colors:[['W','U','B','R','G'],['W','U'],['R']][index],type_line:'Creature',power_toughness:'2/2'}} sourceImageUrl={src}
          variant={mode==='hand'?'hand':'battlefield'} battlefieldVisualMode={mode==='mobile-token'?'mobile-token':'portrait'}
          style={{width:180,height:252}} />
      </div>)}
      {window.__cardArtLoaderPendingFixture && (
        <div data-probe="late" style={{position:'relative',width:180,height:252}}>
          <LoadingCardArt sourceKey={src} src={src} pending={pending} variant="hand" alt="" />
        </div>
      )}
    </main>
  </DragProvider></HoverProvider></ObjectSelectionProvider></GameContext.Provider>;
}
createRoot(document.getElementById('root')).render(<I18nProvider><Fixture /></I18nProvider>);
