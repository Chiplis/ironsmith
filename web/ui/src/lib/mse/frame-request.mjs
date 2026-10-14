import {cardArtCropUrl} from '../card-image-variants.js';
import {selectMsePresentation} from './style-selection.mjs';
import {printingMseCardFields,printingMseFooterFields,printingMseSetFields} from './printing-style.mjs';

// The render request for a card's frame. Card surfaces and background
// preparation build it the same way, so a prepared result is reused as-is.
export function mseFrameRequest(card,{imageUrl,requestedStyle,defaultStyle,faceIndex=0,printing=null,setSymbol=null,availableStyles}={}){
  const explicitStyle=Boolean((requestedStyle&&requestedStyle!=='auto')||(defaultStyle&&defaultStyle!=='auto'));
  const presentation=selectMsePresentation(card,{style:requestedStyle,defaultStyle,faceIndex,printing,availableStyles});
  const {style,styling}=presentation;
  const request={style,snapshot:card,adapter:{styling,faceIndex:card.card_faces&&!['split','adventure'].includes(card.layout)?faceIndex:null,
    set:{...printingMseSetFields(card,printing),...(setSymbol?{symbol:setSymbol}:{})},
    nativeCard:{...printingMseFooterFields(card,printing),...(!explicitStyle?printingMseCardFields(card,printing):{}),...(imageUrl?{image:cardArtCropUrl(imageUrl)}:{})}}};
  return {presentation,request};
}
