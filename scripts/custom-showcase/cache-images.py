"""Cache attributed source prints locally for a CORS-independent showcase.
Run after build-bundled-sets.mjs; reruns reuse downloaded assets.
"""
import concurrent.futures,hashlib,io,json,pathlib,time,urllib.request
from PIL import Image
root=pathlib.Path('web/ui/public/custom-sets');images=root/'images';images.mkdir(exist_ok=True)
manifest=json.loads((root/'index.json').read_text());jobs={};sets={}
for entry in manifest['sets']:
 p=pathlib.Path('web/ui/public')/entry['bundlePath'];data=json.loads(p.read_text());sets[p]=data
 for card in data['cards']:
  for face in card['draft']['faces']:
   url=face.get('sourceImageUrl') or face['imageUrl'];face['sourceImageUrl']=url
   jobs.setdefault(url,[]).append(face)
def fetch(item):
 url,faces=item;key=hashlib.sha256(url.encode()).hexdigest()[:24];dest=images/(key+'.webp');crop=images/(key+'-art.webp')
 try:
  if not dest.exists():
   with urllib.request.urlopen(url,timeout=20) as response: payload=response.read()
   im=Image.open(io.BytesIO(payload)).convert('RGB');im.thumbnail((900,1000));im.save(dest,'WEBP',quality=86,method=4)
  if not crop.exists():
   with Image.open(dest) as im:
    # Standard print art window; keep the complete print as the canonical image.
    w,h=im.size;art=im.crop((int(w*.08),int(h*.13),int(w*.92),int(h*.53)));art.thumbnail((500,365));art.save(crop,'WEBP',quality=86,method=4)
  return url,{'imageUrl':f'custom-sets/images/{dest.name}','artUrl':f'custom-sets/images/{crop.name}'}
 except Exception as e:return url,{'error':str(e)}
results={};started=time.time()
with concurrent.futures.ThreadPoolExecutor(max_workers=16) as pool:
 for url,value in pool.map(fetch,jobs.items()):
  results[url]=value
  if len(results)%500==0:print(f'{len(results)}/{len(jobs)} cached; {sum("error" in v for v in results.values())} failures; {time.time()-started:.0f}s',flush=True)
for url,faces in jobs.items():
 if 'error' not in results[url]:
  for face in faces:face.update(results[url])
for p,data in sets.items():p.write_text(json.dumps(data,ensure_ascii=False,separators=(',',':')))
report={'total':len(results),'cached':sum('error' not in v for v in results.values()),'failures':{k:v['error'] for k,v in results.items() if 'error' in v}}
(root/'image-cache-report.json').write_text(json.dumps(report,indent=2));print(json.dumps({'total':report['total'],'cached':report['cached'],'failures':len(report['failures'])}),flush=True)
