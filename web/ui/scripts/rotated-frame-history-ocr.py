#!/usr/bin/env python3
"""OCR each independently oriented half of a two-ended flip card scan."""
import concurrent.futures,json,pathlib,re,subprocess
from PIL import Image
root=pathlib.Path(__file__).resolve().parents[3]
out=root/'web/ui/test-results/frame-history-final/ocr-rotated';out.mkdir(exist_ok=True)
cache=pathlib.Path('/tmp/ironsmith-frame-history')
def recognize(im,path,box):
 x,y,w,h=box;W,H=im.size
 crop=im.crop((round(x*W),round(y*H),round((x+w)*W),round((y+h)*H)))
 crop.resize((crop.width*3,crop.height*3)).save(path)
 r=json.loads(subprocess.run([str(cache/'recognize'),str(path)],capture_output=True,text=True,check=True).stdout)
 for line in r['lines']:
  for b in [line,*line.get('characters',[])]:
   b['x']=x+b['x']*w;b['y']=y+b['y']*h;b['width']*=w;b['height']*=h
 return r['lines']
def run(c):
 if c['layout']!='flip':return
 for angle in [0,180]:
  dest=out/(c['slug']+f'-{angle}.json')
  if dest.exists() and json.loads(dest.read_text()).get('revision')==3:continue
  im=Image.open(cache/(c['slug']+'-normal.jpg')).rotate(angle)
  path=out/(c['slug']+f'-{angle}.png')
  # Full-image Vision can auto-orient to the other half. Isolate this half.
  lines=recognize(im,path,(0,0,1,.5))
  for line in list(lines):
   if .22<line['y']<.4 and re.search(r'\d+/\d+$',line['text']) and len(line['text'])>8:
    y=max(0,line['y']-.01);h=line['height']+.02
    pieces=recognize(im,out/(c['slug']+f'-{angle}-type.png'),(0,y,.78,h))
    pieces+=recognize(im,out/(c['slug']+f'-{angle}-stats.png'),(.78,y,.22,h))
    if any(re.fullmatch(r'\d+/\d+',p['text']) for p in pieces):lines.remove(line);lines.extend(pieces)
  dest.write_text(json.dumps({'revision':3,'lines':lines}));print(c['name'],angle,len(lines),flush=True)
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
 list(pool.map(run,json.load(open(root/'web/ui/tests/frame-history-corpus.json'))['cases']))
