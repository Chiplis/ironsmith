import json,sys,glob
from PIL import Image, ImageChops, ImageStat
d=sys.argv[1]; m=json.load(open(d+'/meta.json')); b=m.get('box') or {'x':m['x']-m['w']/2,'y':m['y']-m['h']/2,'w':m['w'],'h':m['h']}
files=sorted(glob.glob(d+'/frames/*.jpg'))
cx=b['x']+b['w']/2; cy=b['y']+b['h']/2
K=4  # supersample
tw=b['w']*0.70; th=b['h']*0.55   # inner template (art + title), away from border glow
def crop(im,w,h,size):
    box=(cx-w/2,cy-h/2,cx+w/2,cy+h/2)
    return im.resize(size,Image.BICUBIC,box=box)
base=Image.open(files[0]).convert('L')
size=(int(tw*K/2),int(th*K/2))
tmpl=crop(base,tw,th,size)
scales=[1+i*0.002 for i in range(-10,66)]
def est(im):
    best=None
    for s in scales:
        c=crop(im,tw*s,th*s,size)
        e=ImageStat.Stat(ImageChops.difference(c,tmpl)).rms[0]
        if best is None or e<best[0]: best=(e,s)
    return round(best[1],3)
t0=m['frames'][0]['ts']
ev=[(e['type'],round((e['ts']-t0)*1000)) for e in m['events']]
win=[(te-60,te+800) for _,te in ev]
sc=[]
for f,fr in zip(files,m['frames']):
    t=round((fr['ts']-t0)*1000)
    if any(a<=t<=z for a,z in win): sc.append((t,est(Image.open(f).convert('L'))))
json.dump({'events':ev,'series':sc},open(d+'/series.json','w'))
print('frames',len(files))
for typ,te in ev:
    print(f'--- {typ} at {te}')
    print('   '+'  '.join(f'+{t-te}:{s}' for t,s in sc if te-30<=t<=te+560))
if '-v' in sys.argv:
    for l in m.get('loaf',[]): print('LoAF +%d dur %d'%(round(l[0]-t0*1000), l[1]), [ (x[0][:22],x[1]) for x in l[2][:2]])
