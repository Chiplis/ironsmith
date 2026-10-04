import test from 'node:test';
import assert from 'node:assert/strict';
import {cardPrintingProfile,FORENSIC_PRINTING_BRANCHES,profileSectionInk} from '../src/lib/card-printing-profile.js';
import {cardTypography} from '../src/lib/card-typography.js';
test('chart editions select retro profiles even when frame/date metadata is missing',()=>{
 for(const b of FORENSIC_PRINTING_BRANCHES.filter(b=>!['*','wc','unhinged','unstable','cst'].includes(b.set)&&!['unh','ust'].includes(b.set))){
  const p=cardPrintingProfile({set:b.set});assert.equal(p.era,'retro',b.id);assert.ok(p.chartBranches.includes(b.id),b.id);
 }
 assert.equal(cardPrintingProfile({set:'cst'}).era,'modern');
 assert.equal(cardPrintingProfile({set:'unh'}).era,'modern');
 assert.equal(cardPrintingProfile({set:'ust'}).era,'beleren');
 assert.equal(cardPrintingProfile({set:'wc97'}).treatment,'conventional');
 assert.ok(cardPrintingProfile({set:'wc97',border_color:'gold'}).chartBranches.includes('world-championship'));
});
test('1995 label revision is separate from early engraved labels and from rules ink',()=>{
 for(const set of ['4ed','5ed','6ed','ice','all','chr','bchr','4bb','cst','btd','brb','ath','itp','por','p02','ptk','s99','s00','rqs']){
  for(const colors of [['W'],['U'],['B'],['R'],['G'],['U','W'],[]]){
   const p=cardPrintingProfile({set,frame:'1993',colors});
   assert.equal(profileSectionInk(p,'type'),'light',set);assert.equal(profileSectionInk(p,'name'),'light',set);
   assert.equal(profileSectionInk(p,'rule'),'dark',set);assert.equal(profileSectionInk(p,'flavor'),'dark',set);
  }
 }
 for(const set of ['lea','leb','2ed','3ed','arn','atq','leg','drk','sum','fbb']){
  const p=cardPrintingProfile({set,frame:'1993'});assert.equal(p.ink.title,undefined,set);assert.equal(p.ink.rules,'dark',set);
 }
});
test('special and later frames retain scan-derived ink and printed frame takes precedence over release date',()=>{
 for(const change of [{full_art:true},{border_color:'borderless'},{frame_effects:['showcase']},{layout:'split'}])assert.deepEqual(cardPrintingProfile({set:'ice',frame:'1993',...change}).ink,{});
 for(const set of ['ugl','unh','ust'])assert.deepEqual(cardPrintingProfile({set,frame:'1993'}).ink,{});
 assert.equal(cardTypography({frame:'1993',released_at:'2023-01-01'}).era,'retro');
 assert.equal(cardTypography({set:'ust',frame:'2015'}).era,'beleren');
 assert.deepEqual(cardPrintingProfile({set:'m19',frame:'2015'}).ink,{});
});
test('language and physical-only distinctions remain identifiable without inventing scan evidence',()=>{
 const p=cardPrintingProfile({set:'4ed',lang:'de',border_color:'white'});
 assert.equal(p.lang,'de');assert.ok(!p.chartBranches.includes('alternate-fourth'));
 assert.ok(p.unresolvedPhysicalVariants.includes('fourth-language-variants'));
 assert.ok(cardPrintingProfile({set:'4ed',lang:'en'}).unresolvedPhysicalVariants.includes('alternate-fourth'));
 assert.equal(cardPrintingProfile({set:'3ed',lang:'fr'}).id,'3ed:retro:fr:conventional');
});

test('front border, back border and corner cut are independent of section ink',()=>{
 assert.equal(cardPrintingProfile({set:'cei',border_color:'black'}).backBorder,'gold');
 assert.equal(cardPrintingProfile({set:'cei',border_color:'black'}).frontBorder,'black');
 assert.equal(cardPrintingProfile({set:'lea'}).cornerCut,'alpha');
 for(const border_color of ['white','black','gold'])assert.equal(cardPrintingProfile({set:'4ed',border_color}).ink.type,'light');
 assert.notEqual(cardPrintingProfile({set:'dmr',frame:'1993'}).id,cardPrintingProfile({set:'dmr',frame:'2015'}).id);
});
