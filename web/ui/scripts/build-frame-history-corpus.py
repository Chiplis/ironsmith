#!/usr/bin/env python3
"""Pin a diverse, repeatable sample of standard-size paper frame families.

python3 web/ui/scripts/build-frame-history-corpus.py --cache /tmp/frame-history
Reads the local Scryfall catalog, supplements sparse historical treatments from
Scryfall at <= 5 requests/sec, pins metadata, then downloads scans for offline QA.
No scan is committed; --manifest can select a previously pinned corpus.
"""
import argparse, collections, concurrent.futures, hashlib, json, pathlib, time, urllib.request, urllib.parse
ROOT=pathlib.Path(__file__).resolve().parents[3]
HISTORY='https://magic.wizards.com/en/news/making-magic/frames-reference-2003-01-27'
M15='https://magic.wizards.com/en/news/making-magic/directors-chair-2013-2014-01-06'
FUTURE='https://magic.wizards.com/en/news/feature/the-nonsense-files-the-many-frames-of-mystery-booster-2'
SHOWCASE='https://magic.wizards.com/en/news/feature/collecting-kaldheim-2021-01-07'
LAYOUTS='https://scryfall.com/docs/api/layouts'
MASTERPIECE='https://magic.wizards.com/en/news/making-magic/masterpiece-series-2016-09-12'
# A frame generation, a mechanical layout and a showcase treatment are separate
# axes: memberships deliberately overlap, but each family contains unique cards.
REPRODUCTIONS=['b90faa91-3173-4898-a1fa-0b8e7ce35c72','feaba0f4-de2b-46a5-a728-04c8d699c523']
FAMILIES=[]
def family(key,label,query,predicate,source=HISTORY):
    FAMILIES.append(dict(key=key,label=label,query=query,match=predicate,source=source))
for frame,label in [('1993','Original frame'),('1997','1997 classic frame'),('2003','Eighth Edition modern frame'),('2015','M15 frame'),('future','Future Sight curved frame')]:
    family('frame-'+frame,label,'frame:'+frame,lambda p,f=frame:p.get('frame')==f,FUTURE if frame=='future' else M15 if frame=='2015' else HISTORY)
for effect,label in [('colorshifted','Planar Chaos colorshifted'),('tombstone','Graveyard tombstone'),('miracle','Miracle'),('enchantment','Nyx enchantment'),('devoid','Devoid'),('snow','Snow'),('legendary','Legendary crown'),('extendedart','Extended art')]:
    query='is:nyxtouched' if effect=='enchantment' else 'frame:'+effect
    family('effect-'+effect,label,query,lambda p,e=effect:e in p.get('frame_effects',[]),LAYOUTS)
for border in ['white','borderless']:
    family('border-'+border,border.title()+' border','border:'+border,lambda p,b=border:p.get('border_color')==b,SHOWCASE)
family('full-art-land','Full-art lands','t:land is:fullart',lambda p:p.get('full_art') and 'Land' in p.get('type_line',''),SHOWCASE)
for layout in ['transform','modal_dfc','meld','flip','split','adventure','saga','class','leveler','prototype','mutate','case']:
    family('layout-'+layout,layout.replace('_',' ').title(), 'layout:'+layout,lambda p,l=layout:p.get('layout')==l,LAYOUTS)
for kind,query in [('Planeswalker','t:planeswalker'),('Battle','t:battle'),('Room','t:room'),('Vehicle','t:vehicle'),('Station','keyword:station')]:
    family('type-'+kind.lower(),kind,query,lambda p,k=kind: k in p.get('type_line','') or k in p.get('keywords',[]),LAYOUTS)
family('aftermath','Aftermath split frame','keyword:aftermath',lambda p:'Aftermath' in p.get('keywords',[]),LAYOUTS)
for set_code,label in [('exp','Zendikar Expeditions'),('mps','Kaladesh Inventions'),('mp2','Amonkhet Invocations')]:
    family('masterpiece-'+set_code,label,'set:'+set_code,lambda p,s=set_code:p.get('set')==s,MASTERPIECE)
for sets,key,label in [(['eld','woe'],'storybook','Eldraine storybook'),(['iko'],'comic','Ikoria comic'),(['khm'],'nordic','Kaldheim showcase'),(['neo'],'ninja','Kamigawa showcase'),(['sta'],'archive','Mystical Archive'),(['snc'],'deco','New Capenna showcase')]:
    family('showcase-'+key,label,'('+' or '.join('set:'+s for s in sets)+') is:showcase',lambda p,ss=sets:p.get('set') in ss and 'showcase' in p.get('frame_effects',[]),LAYOUTS)

def eligible(p):
    return p.get('lang')=='en' and 'paper' in p.get('games',[]) and not p.get('oversized') and p.get('layout') not in ['art_series','planar','scheme','vanguard','token','double_faced_token','emblem'] and p.get('border_color') not in ['silver','gold'] and p.get('security_stamp')!='acorn' and p.get('image_status') not in ['placeholder','missing'] and (p.get('image_uris',{}).get('normal') or any(f.get('image_uris',{}).get('normal') for f in p.get('card_faces',[])))
def traits(p):
    color=''.join(p.get('colors',p.get('color_identity',[]))) or 'C'
    text=p.get('oracle_text','') or '\n'.join(f.get('oracle_text','') for f in p.get('card_faces',[]))
    return [f'color:{color}',f'type:{p.get("type_line","").split(" — ")[0]}',f'set:{p.get("set")}',f'frame:{p.get("frame")}',f'layout:{p.get("layout")}',f'border:{p.get("border_color")}',f'length:{min(4,len(text)//150)}',f'mana:{min(6,int(p.get("cmc",0)))}',f'name:{min(3,len(p["name"])//12)}']
def select(pool,count):
    # Greedy inverse-frequency coverage selects different colours, types, text
    # lengths, costs, eras and borders. Hash is a stable, order-independent tie.
    pool=sorted(pool,key=lambda p:hashlib.sha256(p['id'].encode()).hexdigest())
    selected=[];seen=collections.Counter();names=set()
    while pool and len(selected)<count:
        p=max(pool,key=lambda p:sum((3 if t.startswith(('color:','type:','length:')) else 1)/(1+seen[t]) for t in traits(p))-(8 if p.get('oracle_id',p['name']) in names else 0))
        selected.append(p);seen.update(traits(p));names.add(p.get('oracle_id',p['name']));pool=[c for c in pool if c.get('oracle_id',c['name']) not in names]
    return selected
last_request=0
def fetch(url):
    global last_request
    if 'api.scryfall.com' in url:
        time.sleep(max(0,.2-(time.monotonic()-last_request)));last_request=time.monotonic()
    for attempt in range(4):
        try:
            req=urllib.request.Request(url,headers={'User-Agent':'IronsmithFrameHistoryAudit/1.0','Accept':'application/json,image/jpeg,image/svg+xml'})
            with urllib.request.urlopen(req,timeout=45) as r:return r.read()
        except Exception:
            if attempt==3:raise
            time.sleep(2**attempt)
def main():
    parser=argparse.ArgumentParser();parser.add_argument('--cache',required=True);parser.add_argument('--count',type=int,default=25);parser.add_argument('--manifest',default=str(ROOT/'web/ui/tests/frame-history-corpus.json'));parser.add_argument('--refresh',action='store_true');parser.add_argument('--symbols-only',action='store_true');args=parser.parse_args()
    cache=pathlib.Path(args.cache);cache.mkdir(parents=True,exist_ok=True);manifest=pathlib.Path(args.manifest)
    if args.refresh or not manifest.exists():
        all_cards={p['id']:p for p in json.loads((ROOT/'cards.json').read_text()) if eligible(p)}
        groups=[];chosen={}
        for f in FAMILIES:
            pool=[p for p in all_cards.values() if f['match'](p)]
            if len(pool)<args.count:
                q=f'({f["query"]}) game:paper lang:en -is:oversized'
                search=cache/(f['key']+'-search.json')
                if not search.exists():search.write_bytes(fetch('https://api.scryfall.com/cards/search?'+urllib.parse.urlencode({'q':q,'unique':'prints','order':'name'})))
                candidates=json.loads(search.read_text())['data']
                for p in candidates:
                    if eligible(p):all_cards[p['id']]=p
                pool=[p for p in all_cards.values() if f['match'](p)]
            sample=select(pool,args.count)
            groups.append({k:v for k,v in f.items() if k!='match'}|{'candidateCount':len(pool),'distinctCards':len({p.get('oracle_id',p['name']) for p in pool}),'selected':len(sample),'ids':[p['id'] for p in sample]})
            for p in sample:
                chosen[p['id']]=p
                (cache/(p['id']+'.json')).write_text(json.dumps(p,ensure_ascii=False))
            print(f'{f["key"]}: {len(sample)}/{len(pool)}',flush=True)
        # Always keep the user-reported reproduction, regardless of sampling.
        for reproduction_id in REPRODUCTIONS:
            meta=cache/(reproduction_id+'.json')
            if not meta.exists():meta.write_bytes(fetch('https://api.scryfall.com/cards/'+reproduction_id))
            chosen[reproduction_id]=json.loads(meta.read_text())
        cases=[]
        for id,p in sorted(chosen.items()):
            faces=[(None,p)] if p.get('image_uris') else list(enumerate(p.get('card_faces',[])))
            for index,face in faces:
                if not face.get('image_uris',{}).get('normal'):continue
                cases.append({'id':id,'face':index,'slug':id+(f'-{index}' if index is not None else ''),'name':face['name'],'set':p['set'],'number':p['collector_number'],'families':[g['key'] for g in groups if id in g['ids']],'source':face['image_uris']['normal'],'art':face['image_uris'].get('art_crop'),'frame':p.get('frame'),'layout':p.get('layout')})
        manifest.write_text(json.dumps({'version':1,'reproductions':REPRODUCTIONS,'targetPerFamily':args.count,'scope':'English, standard-size, paper playable cards; no novelty/oversized/art-series/token cards','families':groups,'cases':cases},indent=2)+'\n')
    corpus=json.loads(manifest.read_text());jobs=[]
    sets_path=cache/'sets.json'
    if not sets_path.exists():sets_path.write_bytes(fetch('https://api.scryfall.com/sets'))
    sets={s['code']:s for s in json.loads(sets_path.read_text())['data']}
    for code in sorted({c['set'] for c in corpus['cases']}):
        if code not in sets:continue
        value=sets[code];(cache/(code+'-set.json')).write_text(json.dumps(value))
        symbol=cache/(code+'.svg')
        if not symbol.exists():jobs.append((symbol,value['icon_svg_uri']))
    for c in corpus['cases']:
        meta=cache/(c['id']+'.json')
        if not meta.exists():meta.write_bytes(fetch('https://api.scryfall.com/cards/'+c['id']))
        for kind,url in ([] if args.symbols_only else [('normal',c['source']),('art_crop',c['art'])]):
            path=cache/(c['slug']+'-'+kind+'.jpg')
            if url and not path.exists():jobs.append((path,url))
    def download(job):
        path,url=job;path.write_bytes(fetch(url));return path.name
    with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
        for n,name in enumerate(pool.map(download,jobs)):
            if n%25==0:print(f'Images {n+1}/{len(jobs)} {name}',flush=True)
    print(f'{len(corpus["families"])} families, {len(corpus["cases"])} faces cached in {cache}',flush=True)
if __name__=='__main__':main()
