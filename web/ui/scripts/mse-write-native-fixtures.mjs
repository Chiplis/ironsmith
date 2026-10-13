import {engineDoubleFacesToMse} from '../src/lib/mse/engine-card.mjs';
import {selectMseStyle} from '../src/lib/mse/style-selection.mjs';
import {mkdir,writeFile} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import fixtures from './mse-render-fixtures.mjs';
import {corpusStore} from './mse-corpus-store.mjs';
import upstream from '../src/lib/mse/upstream.mjs';

// Export our populated rendering fixtures as native directory sets. These are
// reference-test inputs, including scalar linked-card UIDs and per-card styles.
// Images, symbols, and arbitrary game-specific values still need serialization.
const [corpus,output]=process.argv.slice(2);
if(!corpus||!output)throw new Error('Usage: mse-write-native-fixtures.mjs CORPUS OUTPUT_DIRECTORY');
const store=await corpusStore(corpus),root=resolve(output),results=[];
const scalar=(key,value,depth=0)=>{
  if(value===null||value===undefined)return '';
  if(!['string','number','boolean'].includes(typeof value))throw new Error(`Fixture field ${key} needs a native value serializer`);
  if(!/^[\w -]+$/.test(key))throw new Error(`Invalid fixture field ${key}`);
  const prefix='\t'.repeat(depth)+key.replaceAll('_',' ');
  const text=typeof value==='boolean'?(value?'yes':'no'):String(value).replace(/\r\n?|\n\r/g,'\n');
  if(text.includes('\n')||/^\s/.test(text))return prefix+':\n'+text.split('\n').map(line=>'\t'.repeat(depth+1)+line).join('\n')+'\n';
  return prefix+': '+text+'\n';
};
const fields=(values,depth)=>Object.entries(values||{}).map(([key,value])=>scalar(key,value,depth)).join('');
await mkdir(root,{recursive:true});
for(const fixture of fixtures){
  const style=(await store.package(fixture.style)).document;
  const gameName=style.text('game'),game=(await store.package(gameName+'.mse-game')).document;
  const linked=fixture.snapshot?engineDoubleFacesToMse(fixture.snapshot,{faceIndex:fixture.faceIndex}):null;
  const context=linked?linked.cards[linked.selected]:fixture.context;
  if(Object.keys(context.extraCard||{}).length)throw new Error('Extra-card fixture serialization is not implemented');
  let source=scalar('mse version','2.6.4')+scalar('game',gameName)+scalar('game version',game.text('version'))
    +scalar('stylesheet',fixture.style)+scalar('stylesheet version',style.text('version'));
  if(Object.keys(context.set||{}).length)source+='set info:\n'+fields(context.set,1);
  for(const [index,values] of (linked?.cards||[context]).entries()){
    source+='card:\n';
    if(linked){
      const faceStyle=selectMseStyle(fixture.snapshot,{faceIndex:index});
      const document=(await store.package(faceStyle)).document;
      source+=scalar('stylesheet',faceStyle,1)+scalar('stylesheet version',document.text('version'),1)+scalar('uid',values.uid,1);
    }
    if(Object.keys(values.styling||{}).length)source+=scalar('has styling',true,1)+'\tstyling data:\n'+fields(values.styling,2);
    source+=fields(values.card,1);
  }
  const directory=join(root,fixture.id+'.mse-set');
  await mkdir(directory,{recursive:true});await writeFile(join(directory,'set'),source);
  results.push({id:fixture.id,style:fixture.style,path:directory,cardIndex:linked?.selected??0,expectedDimensions:fixture.dimensions,expectedText:fixture.text});
}
await writeFile(join(root,'manifest.json'),JSON.stringify({upstream,scope:'Native reference inputs; export and visual comparison still required',fixtures:results},null,2)+'\n');
console.log(JSON.stringify({directory:root,fixtures:results.length}));
