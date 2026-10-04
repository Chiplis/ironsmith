// Front-rendering consequences of the revised forensic identification chart v4
// (April 2019). Edition identity comes from catalogue metadata; back-side dots,
// UV response, corner cuts and individual misprints cannot be inferred from a
// front scan. Preserve those as diagnostic distinctions, not guessed geometry.
export const FORENSIC_PRINTING_BRANCHES = [
  ['international-collectors','cei','gold-back','back edition text'],
  ['collectors','ced','gold-back','back edition text'],
  ['world-championship','wc','gold-front','commemorative deck'],
  ['unglued','ugl','silver','set symbol'],['unhinged','unh','silver','set symbol'],
  ['unstable','ust','silver-or-special','UST footer / set symbol'],
  ['unlimited','2ed','white-undated','artist baseline centred'],
  ['revised','3ed','white-undated','artist on baseline; language and back-dot variants'],
  ['fourth','4ed','white-dated','1995; language and back-dot variants'],
  ['fifth','5ed','white-dated','1997; simplified Chinese 1993–1998'],
  ['sixth','6ed','white-dated','1993–1999 collector number; simplified Chinese 1993–2000'],
  ['summer','sum','white-dated','1994 copyright position'],
  ['comic-promo','pmei','white-dated','1994 copyright position; Fireball / Blue Elemental Blast'],
  ['alliances','all','white-dated-or-black-symbol','1996 set icon'],
  ['introductory-two-player','itp','white-dated','1996 no set icon; some 1995 printings'],
  ['portal','por','white-dated-or-black-symbol','1997 set icon; simplified Chinese 1998'],
  ['portal-second-age','p02','white-dated','1993–1998 set icon'],
  ['anthologies','ath','white-dated','1993–1998 no/other icon; reprint'],
  ['coldsnap-theme-decks','cst','white-dated','1993–2006 Ice Age / Alliances icon; reprint'],
  ['starter-2000','s00','white-dated','1993–2000 no icon'],
  ['beatdown','btd','white-dated','1993–2000 set icon; reprint'],
  ['battle-royale','brb','white-dated','1993–1999 no/other icon; reprint'],
  ['ice-age','ice','white-dated-or-black-symbol','1995 set icon'],
  ['chronicles','chr','white-dated-or-black-symbol','1995 reused icons; Japanese black border'],
  ['renaissance','ren','black-symbol','language-specific reused expansion icons'],
  ['rinascimento','rin','black-symbol','Italian reused expansion icons'],
  ['fourth-foreign-black-border','4bb','black-dated','language; Fourth Edition'],
  ['chronicles-foreign-black-border','bchr','black-symbol','Japanese reused icons'],
  ['foreign-black-border','fbb','black-symbol-or-undecorated','German/Italian 1994; French 1994/1995'],
  ['alpha','lea','black-no-symbol','rounded corners'],
  ['beta','leb','black-no-symbol','sharper corners; Reconstruction absent'],
  ['antiquities-misprint','atq','black-no-symbol','Reconstruction'],
  ['renaissance-misprint','ren','black-no-symbol','dated; Winter Blast / Rafale Hivernale'],
  ['arabian-nights','arn','black-symbol','scimitar icon'],
  ['antiquities','atq','black-symbol','anvil icon'],
  ['legends','leg','black-symbol','column icon'],['the-dark','drk','black-symbol','crescent icon'],
  ['alternate-fourth','4ed','white-dated','English; backside printing pattern / UV response'],
  ['rivals-quick-start','rqs','white-dated','1995 backside black spot'],
  ['fourth-language-variants','4ed','white-dated','French wording; German umlauts; Italian back dot'],
  ['revised-language-variants','3ed','white-undated','French wording; German umlauts; Italian back dot'],
  ['other-set','*','any','set symbol / catalogue identity'],
  ['promo-prerelease','*','any','multiple symbols / catalogue promo flags'],
 ].map(([id,set,branch,evidence])=>({id,set,branch,evidence,
  languages:id==='alternate-fourth'?['en']:id.endsWith('language-variants')?['fr','de','it']:undefined,
  cardNames:id==='antiquities-misprint'?['Reconstruction']:id==='renaissance-misprint'?['Winter Blast','Rafale Hivernale']:id==='comic-promo'?['Fireball','Blue Elemental Blast']:undefined,
  physicalEvidenceRequired:['alternate-fourth','fourth-language-variants','revised-language-variants'].includes(id),
}));

const chartSets = new Set(FORENSIC_PRINTING_BRANCHES.map(b=>b.set).filter(s=>!['*','wc'].includes(s)));
const specialEffects = new Set(['showcase','extendedart','borderless','shatteredglass']);
export function cardPrintingProfile(printing = {}) {
  const set=String(printing.set||'').toLowerCase(), lang=printing.lang||'en';
  const frame=String(printing.frame||'');
  const date=/^\d{4}-\d{2}-\d{2}$/.test(printing.released_at||'')?printing.released_at:null;
  const era=({'1993':'retro','1997':'retro','2003':'modern','2015':'beleren',future:'future'})[frame]
    || (date?date<'2003-07-28'?'retro':date<'2014-07-18'?'modern':'beleren':['unh','cst'].includes(set)?'modern':set==='ust'?'beleren':chartSets.has(set)||/^wc\d{2}$/.test(set)?'retro':'beleren');
  const conventional=!printing.full_art && printing.border_color!=='borderless'
    && !(printing.frame_effects||[]).some(e=>specialEffects.has(e))
    && !['saga','class','split','flip','planar','scheme','adventure','leveler','prototype','augment','art_series'].includes(printing.layout);
  const branches=FORENSIC_PRINTING_BRANCHES.filter(b=>(b.set===set||b.set==='wc'&&/^wc\d{2}$/.test(set))
    && (!b.languages || !printing.lang || b.languages.includes(printing.lang))
    && (!b.cardNames || !printing.name || b.cardNames.includes(printing.name) || b.cardNames.includes(printing.printed_name)));
  if(!branches.length)branches.push(FORENSIC_PRINTING_BRANCHES.find(b=>b.id===(printing.promo?'promo-prerelease':'other-set')));
  const treatment=!conventional?'special':printing.border_color==='gold'?'commemorative':printing.border_color==='silver'?'silver':'conventional';
  // The 1995 revision changed the label fill even on white cards. Earlier
  // editions use dark/grey cores with pale edging; infer those from the scan.
  // Silver-border joke cards can have unusual section treatments.
  const standardRetro=era==='retro'&&conventional&&!['ugl','unh','ust'].includes(set);
  const paleLabels=standardRetro && (['4ed','4bb','5ed','6ed','ice','all','chr','bchr','cst','btd','brb','ath','itp','por','p02','ptk','s99','s00','rqs'].includes(set)
    || date && date>='1995-01-01');
  const ink=standardRetro?{rules:'dark',flavor:'dark',...(paleLabels?{title:'light',type:'light',stats:'light'}:{})}:{};
  return {id:`${set||'unknown'}:${frame||era}:${lang}:${treatment}`,era,set,lang,treatment,conventional,
    chartBranches:branches.map(b=>b.id),ink,
    frontBorder:printing.border_color||null,
    backBorder:['cei','ced'].includes(set)?'gold':'standard',
    cornerCut:set==='lea'?'alpha':set==='cei'||set==='ced'?'square':'standard',
    labelTreatment:paleLabels?'pale-shadow':standardRetro?'scan-engraving':'scan',
    geometry:era==='retro'?'integrated-labels':era==='future'?'future-curved':'enclosed-labels',
    unresolvedPhysicalVariants:branches.filter(b=>b.physicalEvidenceRequired).map(b=>b.id),
  };
}
export const profileSectionInk=(profile,section)=>profile?.ink?.[section==='name'?'title':section==='rule'?'rules':section];

// Publish known palette values even when scan registration cannot yield a mask.
// A textured source still needs readable live text in that placement fallback.
export function printingProfileInkStyle(profile) {
  const style={};
  for(const section of ['title','type','rules','stats']) {
    const ink=profileSectionInk(profile,section);
    if(!ink)continue;
    style[`--sampled-${section}-ink`]=ink==='light'?'rgb(255,255,255)':'rgb(0,0,0)';
    if(ink==='light')style[`--sampled-${section}-shadow`]='.035em .035em .025em rgb(0,0,0)';
  }
  return style;
}
