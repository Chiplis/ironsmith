import {requestInspectorDetails} from '@/lib/inspector-details-cache';
import {useEffect,useRef} from 'react';
import {useGame} from '@/context/GameContext';
import {useI18n} from '@/i18n/I18nContext';
import {CardGlintPreparer,cardGlintZoneCards,prepareCardGlintFace} from '@/lib/card-glint/preparation.mjs';
import {loadCardGlint} from '@/lib/card-glint/session.mjs';

export default function CardGlintZonePreparation(){
  const {state,game}=useGame(), {locale}=useI18n(), preparer=useRef(null);
  const current=useRef({state,game});
  current.current={state,game};
  useEffect(()=>{
    // Start the library/font stylesheet while the initial board is loading.
    loadCardGlint().catch(()=>{});
    const value=new CardGlintPreparer({prepare:card=>prepareCardGlintFace(card,{locale,resolveDetails:card=>{
      const {game,state}=current.current;
      return game && card.id!=null ? requestInspectorDetails(game,state,card.id) : null;
    }})});
    preparer.current=value;
    return ()=>{value.dispose();if(preparer.current===value)preparer.current=null;};
  },[locale,game]);
  useEffect(()=>{preparer.current?.update(cardGlintZoneCards(state));},[state,locale,game]);
  return null;
}
