#!/usr/bin/env python3
"""Apply manually read badge values only to the exact reviewed crop pixels."""
import argparse,hashlib,json,pathlib
from PIL import Image
from card_badge_geometry import stat_badge_crop
p=argparse.ArgumentParser();p.add_argument('--reviews',required=True,type=pathlib.Path);a=p.parse_args()
root=pathlib.Path(__file__).resolve().parents[3];cache=pathlib.Path('/tmp/ironsmith-frame-history');out=root/'web/ui/test-results/frame-history-final/ocr-extra'
cases={c['slug']:c for c in json.load(open(root/'web/ui/tests/frame-history-corpus.json'))['cases']}
for review in json.load(open(a.reviews)):
 c=cases[review['slug']];path=out/(c['slug']+'.png')
 if hashlib.sha256(path.read_bytes()).hexdigest()!=review['cropSha256']:raise ValueError('Reviewed pixels changed: '+c['slug'])
 printing=json.load(open(cache/(c['id']+'.json')));face=(printing.get('card_faces') or [printing])[c.get('face') or 0]
 if 'lines' in review:
  expected=str(face.get('power'))+'/'+str(face.get('toughness'))
  if review['expected']!=expected or [l['text'] for l in review['lines']]!=expected.split('/'):raise ValueError('Reviewed stats differ from metadata')
  for line in review['lines']:
   if not (.8<line['x']<.95 and .75<line['y']<.96 and 0<line['width']<.1 and 0<line['height']<.1):raise ValueError('Invalid reviewed stat bounds')
  (out/(c['slug']+'.json')).write_text(json.dumps({'revision':3,'lines':review['lines'],'manualReview':review}))
  print(c['name'],expected)
  continue
 if review['digit']!=face.get('defense'):raise ValueError('Badge value differs from metadata: '+c['slug'])
 crop,box,(W,H)=stat_badge_crop(Image.open(cache/(c['slug']+'-normal.jpg')),True)
 glyph=crop.convert('L').point(lambda v:255 if v<100 else 0).getbbox()
 if not glyph:raise ValueError('No badge ink')
 x=(box[0]+glyph[0]/5)/W;y=(box[1]+glyph[1]/5)/H;w=(glyph[2]-glyph[0])/5/W;h=(glyph[3]-glyph[1])/5/H
 line=dict(text=review['digit'],confidence=1,x=y,y=1-x-w,width=h,height=w)
 (out/(c['slug']+'.json')).write_text(json.dumps({'revision':2,'lines':[line],'manualReview':review}))
 print(c['name'],review['digit'])
