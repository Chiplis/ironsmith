#!/usr/bin/env python3
"""Independent OCR residual check on changed-text registrations, not screenshots.
Recognizable source text remaining inside a replaced field is a review failure.
"""
import argparse,concurrent.futures,difflib,hashlib,json,pathlib,re,subprocess
from PIL import Image
p=argparse.ArgumentParser();p.add_argument('--output',required=True,type=pathlib.Path);p.add_argument('--recognizer',required=True);a=p.parse_args()
root=pathlib.Path(__file__).resolve().parents[3]
s=(root/'web/ui/src/lib/card-region-history.generated.js').read_text();registrations=json.loads(s[s.index('[\n'):s.rindex(';')]);corpus=json.loads((root/'web/ui/tests/frame-history-corpus.json').read_text())['cases'];rs={next(c['slug'] for c in corpus if c['id']==r['id'] and c['face']==r['face']):r for r in registrations};results=json.loads((a.output/'results.json').read_text());store=a.output/'ocr';store.mkdir(exist_ok=True)
def norm(s):return re.sub('[^a-z0-9]+',' ',s.lower()).strip()
def check(r):
 if r['slug'] not in rs:return r
 registration=rs[r['slug']]
 path=a.output/(r['slug']+'-mask.png');dest=store/(r['slug']+'.json')
 if not path.exists():return r
 digest=hashlib.sha256(path.read_bytes()).hexdigest()
 if dest.exists() and (old:=json.loads(dest.read_text())).get('sha256')==digest and old.get('revision')==2:lines=old['lines']
 else:
  if registration['layout']=='flip':
   image=Image.open(path);lines=[]
   for angle in [0,180]:
    oriented=image.rotate(angle);cut=round(oriented.height*.5);crop=store/(r['slug']+f'-{angle}.png');oriented.crop((0,0,oriented.width,cut)).save(crop)
    rows=json.loads(subprocess.run([a.recognizer,str(crop)],capture_output=True,text=True,check=True).stdout)['lines']
    for line in rows:
     line['y']*=cut/oriented.height;line['height']*=cut/oriented.height
     if angle:line['x']=1-line['x']-line['width'];line['y']=1-line['y']-line['height']
    lines.extend(rows)
   value={'lines':lines}
  else:value=json.loads(subprocess.run([a.recognizer,str(path)],capture_output=True,text=True,check=True).stdout);lines=value['lines']
  value['sha256']=digest;value['revision']=2;dest.write_text(json.dumps(value))
 findings=[];registration=rs[r['slug']]
 replaced=[f['kind'] for f in r.get('fields',[]) if f['replaced']]
 replaced_indices={f['fieldIndex'] for f in r.get('fields',[]) if f['replaced'] and 'fieldIndex' in f}
 indexed=any('fieldIndex' in f for f in r.get('fields',[]))
 for index,field in enumerate(registration['fields']):
  if (index not in replaced_indices if indexed else field['kind'] not in replaced):continue
  for source in field['lines']:
   for line in lines:
    text=norm(line['text']);before=norm(source['text'])
    if len(text)<6 or line['confidence']<.5:continue
    if abs(line['y']-source['y'])>.022:continue
    if line['x']>source['x']+source['width'] or line['x']+line['width']<source['x']:continue
    if difflib.SequenceMatcher(None,text,before).ratio()>.65:findings.append(dict(kind=field['kind'],text=line['text'],source=source['text']))
 # Check the entire live body region against independent original OCR too.
 # Unassigned printed lines must not escape merely because the generator missed them.
 originals=json.loads((root/'web/ui/test-results/frame-history-final/ocr'/(r['slug']+'-original.json')).read_text())['lines']
 if registration.get('rotation')==90:originals=[{**l,'x':1-l['y']-l['height'],'y':l['x'],'width':l['height'],'height':l['width']} for l in originals]
 if registration['layout']=='flip':
  for angle in [0,180]:
   extra=root/'web/ui/test-results/frame-history-final/ocr-rotated'/(r['slug']+f'-{angle}.json')
   if extra.exists():
    originals.extend([{**l,'x':1-l['x']-l['width'],'y':1-l['y']-l['height']} if angle else l for l in json.loads(extra.read_text())['lines']])
 body=[f['region'] for f in registration['fields'] if f['kind']=='rule' and f.get('region')]
 preserved=[f for i,f in enumerate(registration['fields']) if (i not in replaced_indices if indexed else not any(x['kind']==f['kind'] and x['replaced'] for x in r.get('fields',[])))]
 for line in lines:
  if len(norm(line['text']))<6 or line['confidence']<.5:continue
  cx=line['x']+line['width']/2;cy=line['y']+line['height']/2
  if not any(b['x']<=cx<=b['x']+b['width'] and b['y']<=cy<=b['y']+b['height'] for b in body):continue
  if any(f.get('bounds') and f['bounds']['y']<=cy<=f['bounds']['y']+f['bounds']['height'] for f in preserved):continue
  for source in originals:
   if abs(line['y']-source['y'])<.018 and difflib.SequenceMatcher(None,norm(line['text']),norm(source['text'])).ratio()>.7:
    finding=dict(kind='body',text=line['text'],source=source['text'])
    if not any(f['text']==finding['text'] for f in findings):findings.append(finding)
 r['residualFindings']=findings;return r
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
 reviewed=[]
 for i,r in enumerate(pool.map(check,results)):
  reviewed.append(r)
  if i%50==0:print(i+1,len(results),flush=True)
(a.output/'review.json').write_text(json.dumps(reviewed,indent=2));print('Residual candidates:',sum(bool(r.get('residualFindings')) for r in reviewed))
