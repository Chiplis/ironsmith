import {useEffect,useRef} from 'react';
import {useGame} from '@/context/GameContext';
import {useMseRenderer} from '@/context/MseRendererContext';
import {MseCardPreparer,publicZoneCards} from '@/lib/mse-card-preparation';

// Keeps every public-zone card's frame prepared in the background.
export default function MseZonePreparation(){
  const config=useMseRenderer(),{state}=useGame();
  const session=config?.session,defaultFrames=config?.defaultFrames,defaultStyle=config?.defaultStyle;
  const preparer=useRef(null);
  useEffect(()=>{
    if(!session?.renderer?.render)return undefined;
    const value=new MseCardPreparer({session,defaultFrames,defaultStyle});
    preparer.current=value;
    return ()=>{value.dispose();if(preparer.current===value)preparer.current=null;};
  },[session,defaultFrames,defaultStyle]);
  useEffect(()=>{preparer.current?.update(publicZoneCards(state));},[state,session,defaultFrames,defaultStyle]);
  return null;
}
