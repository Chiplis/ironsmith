#!/usr/bin/env python3
"""Read tiny lower-right stat badges independently at a larger OCR scale."""
import argparse,collections,concurrent.futures,json,pathlib,subprocess
from PIL import Image
from card_badge_geometry import stat_badge_crop
p=argparse.ArgumentParser();p.add_argument('--recognizer',required=True);p.add_argument('--only-rejected',action='store_true');a=p.parse_args();root=pathlib.Path(__file__).resolve().parents[3];cache=pathlib.Path('/tmp/ironsmith-frame-history');out=root/'web/ui/test-results/frame-history-final/ocr-extra';out.mkdir(exist_ok=True)
cases=json.load(open(root/'web/ui/tests/frame-history-corpus.json'))['cases']
if a.only_rejected:
 rejected={r['slug'] for r in json.load(open(root/'web/ui/test-results/frame-history-final/registration-review.json'))['rejected']}
 cases=[c for c in cases if c['slug'] in rejected]
def run(c):
 printing=json.load(open(cache/(c['id']+'.json')));face=printing.get('card_faces',[printing])[c.get('face') or 0]
 expected=face.get('loyalty',face.get('defense'))
 if expected is None and face.get('power') is not None:expected=str(face['power'])+'/'+str(face['toughness'])
 if expected is None:return
 rotated=face.get('defense') is not None
 dest=out/(c['slug']+'.json')
 revision=3 if '/' in str(expected) else 2 if rotated else 1
 if dest.exists() and json.load(open(dest)).get('revision')==revision:return
 im=Image.open(cache/(c['slug']+'-normal.jpg'))
 crop,box,(W,H)=stat_badge_crop(im,rotated)
 if '/' in str(expected):
  W,H=im.size;box=(int(.72*W),int((.76 if printing.get('set')=='mp2' else .84)*H),W,int(.98*H));crop=im.crop(box).resize(((box[2]-box[0])*5,(box[3]-box[1])*5))
 path=out/(c['slug']+'.png')
 crop.save(path)
 data=json.loads(subprocess.run([a.recognizer,str(path)],capture_output=True,text=True,check=True).stdout);lines=[]
 for l in data['lines']:
  values=str(expected).split('/') if printing.get('set')=='mp2' and '/' in str(expected) else [str(expected)]
  if l['text'].strip(' ,;:') not in values:continue
  if rotated and l['text']!=str(expected):
   glyph=next((g for g in l.get('characters',[]) if g['text']==str(expected)),None)
   if not glyph:continue
   l={**l,**glyph}
  l['x']=(box[0]+l['x']*(box[2]-box[0]))/W;l['width']*= (box[2]-box[0])/W
  l['y']=(box[1]+l['y']*(box[3]-box[1]))/H;l['height']*= (box[3]-box[1])/H
  if rotated:l={**l,'x':l['y'],'y':1-l['x']-l['width'],'width':l['height'],'height':l['width']}
  lines.append(l)
 dest.write_text(json.dumps({'lines':lines,'revision':revision}));print(c['name'],len(lines),flush=True)
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:list(pool.map(run,cases))
