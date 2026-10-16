import {useEffect,useRef} from 'react';
import {useGame} from '@/context/GameContext';
import {useI18n} from '@/i18n/I18nContext';
import {CardGlintPreparer,cardGlintZoneCards,prepareCardGlintFace} from '@/lib/card-glint/preparation.mjs';
import {loadCardGlint} from '@/lib/card-glint/session.mjs';

export default function CardGlintZonePreparation(){
  const {state}=useGame(), {locale}=useI18n(), preparer=useRef(null);
  useEffect(()=>{
    // Start the library/font stylesheet while the initial board is loading.
    loadCardGlint().catch(()=>{});
    const value=new CardGlintPreparer({prepare:card=>prepareCardGlintFace(card,{locale})});
    preparer.current=value;
    return ()=>{value.dispose();if(preparer.current===value)preparer.current=null;};
  },[locale]);
  useEffect(()=>{preparer.current?.update(cardGlintZoneCards(state));},[state,locale]);
  return null;
}
