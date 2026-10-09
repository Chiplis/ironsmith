#!/usr/bin/env python3
"""Independently compare printed OCR headers with renderer geometry.
Uses the existing local Vision recognizer; never sends scans to an OCR service.
Run after/during the browser audit. Reuses OCR only for identical file hashes.
"""
import argparse, concurrent.futures, difflib, hashlib, json, pathlib, re, subprocess
ROOT=pathlib.Path(__file__).resolve().parents[3]
def normalized(s):return re.sub(r'[^a-z0-9]+',' ',s.lower()).strip()
def ratio(a,b):return difflib.SequenceMatcher(None,normalized(a),normalized(b)).ratio()
def main():
 p=argparse.ArgumentParser();p.add_argument('--cache',required=True);p.add_argument('--output',default=str(ROOT/'web/ui/test-results/frame-history'));p.add_argument('--recognizer',required=True);a=p.parse_args()
 cache=pathlib.Path(a.cache);out=pathlib.Path(a.output);store=out/'ocr';store.mkdir(exist_ok=True)
 results=json.loads((out/'results.json').read_text());jobs=[]
 for r in results:
  for suffix,path in [('original',cache/(r['slug']+'-normal.jpg')),('mask',out/(r['slug']+'-mask.png'))]:
   if path.exists():jobs.append((r['slug']+'-'+suffix,path))
 def recognize(job):
  key,path=job;digest=hashlib.sha256(path.read_bytes()).hexdigest();dest=store/(key+'.json')
  if dest.exists():
   previous=json.loads(dest.read_text())
   if previous.get('sha256')==digest:return key,previous['lines']
  proc=subprocess.run([a.recognizer,str(path)],capture_output=True,text=True,check=True)
  value=json.loads(proc.stdout);value['sha256']=digest;dest.write_text(json.dumps(value));return key,value['lines']
 observed={}
 with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
  for i,(key,lines) in enumerate(pool.map(recognize,jobs)):
   observed[key]=lines
   if i%50==0:print(f'OCR {i+1}/{len(jobs)}',flush=True)
 if not any(observed.values()):raise RuntimeError('OCR returned no text; check Vision process permissions')
 for r in results:
  printing=json.loads((cache/(r['id']+'.json')).read_text());face=printing if r.get('face') is None else printing['card_faces'][r['face']]
  original=observed.get(r['slug']+'-original',[]);clean=observed.get(r['slug']+'-mask',[])
  findings=[]
  for kind,key,field in [('title','name','titleBounds'),('type','type_line','typeBounds')]:
   text=face.get('flavor_name') if kind=='title' and face.get('flavor_name') else face.get(key,'')
   # Canonical subtitles and rules references are not the printed header.
   candidates=[l for l in original if l['confidence']>=.5 and ratio(text,l['text'])>=.72 and (l['y']<.22 if kind=='title' else .25<l['y']<.75)]
   if not candidates:continue
   printed=max(candidates,key=lambda l:ratio(text,l['text']))
   box=r.get(field)
   if box and r.get('mode')=='masked':
    dx=box['x']-printed['x']*488;dy=box['y']-printed['y']*680
    # OCR includes anti-aliasing/descenders differently. Eight scan pixels
    # tolerates that noise but catches the 23px curved-rim error.
    if abs(dx)>8 or abs(dy)>8:findings.append({'kind':kind+'-alignment','dx':round(dx,1),'dy':round(dy,1),'printed':printed,'detected':box})
   if r.get('mode')=='masked':
    for line in clean:
     if line['confidence']>=.5 and len(normalized(line['text']))>=4 and ratio(text,line['text'])>.65 and abs(line['y']-printed['y'])<.06:
      findings.append({'kind':kind+'-residual','text':line['text']})
  if r.get('mode')=='masked':
   rules=r.get('boxes',{}).get('rules')
   if rules:
    for line in clean:
     if line['confidence']<.6 or len(normalized(line['text']))<5:continue
     if line['y']*680<rules['y'] or (line['y']+line['height'])*680>rules['y']+rules['height']:continue
     if any(ratio(line['text'],o['text'])>.7 and abs(line['y']-o['y'])<.06 for o in original):findings.append({'kind':'rules-residual','text':line['text']})
  r['ocrFindings']=findings
 (out/'review.json').write_text(json.dumps(results,indent=2))
 print('OCR reviewed',len(results),'faces;',sum(bool(r['ocrFindings']) for r in results),'with geometry/residual candidates')
if __name__=='__main__':main()
