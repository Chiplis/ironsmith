#!/usr/bin/env python3
"""Build validated printing registrations from the frame corpus's cached OCR.

python3 web/ui/scripts/register-frame-history.py --cache /tmp/frame-history \
  --audit web/ui/test-results/frame-history-final
Existing hand-reviewed registrations take precedence at runtime. Incomplete or
ambiguous observations stay in the review queue; no card-name branches are used.
"""
import argparse, collections, difflib, functools, hashlib, importlib.util, json, pathlib, re
ROOT=pathlib.Path(__file__).resolve().parents[3]
spec=importlib.util.spec_from_file_location('registration',pathlib.Path(__file__).parent/'card-layouts/register.py')
registration=importlib.util.module_from_spec(spec);spec.loader.exec_module(registration)
registration.similarity=functools.lru_cache(maxsize=50000)(registration.similarity)
def normalized(text):return registration.normalized(text)
def similarity(a,b):return difflib.SequenceMatcher(None,normalized(a),normalized(b)).ratio()
def bounds(lines):
 x=min(l['x'] for l in lines);y=min(l['y'] for l in lines)
 return dict(x=x,y=y,width=max(l['x']+l['width'] for l in lines)-x,height=max(l['y']+l['height'] for l in lines)-y)
def overlap(a,b):
 return max(0,min(a['x']+a['width'],b['x']+b['width'])-max(a['x'],b['x']))*max(0,min(a['y']+a['height'],b['y']+b['height'])-max(a['y'],b['y']))
def contextual_registration(case, printing, observations):
 fields=registration.fields_for(printing,case.get('face'))
 if printing.get('textless'):fields=[f for f in fields if f['kind'] not in ['rule','flavor']]
 faces=printing.get('card_faces') or [printing]
 for i,face_data in enumerate(faces):
  if case.get('face') is not None and case['face']!=i:continue
  if face_data.get('loyalty') is not None:fields.append(dict(kind='stats',text=face_data['loyalty'],face=i,lines=[]))
  if face_data.get('defense') is not None:fields.append(dict(kind='stats',text=face_data['defense'],face=i,lines=[]))
 if case.get('face') is not None:
  for f in fields:f['face']=case['face']
 rows=[dict(l) for l in observations if l['confidence']>=.25 and l['y']<.93]
 used=set()
 if case['layout']=='split':
  for f in fields:
   if f['kind']!='rule' or not f['text'].startswith('Fuse '):continue
   if any(g.get('sharedRule') and g['text']==f['text'] for g in fields):f['sharedDuplicate']=True;continue
   candidates=[(i,l) for i,l in enumerate(rows) if l['width']>.5 and l['text'].startswith('Fuse ') and similarity(l['text'],f['text'])>.7]
   if candidates:
    i,l=max(candidates,key=lambda p:similarity(p[1]['text'],f['text']));f['lines']=[l];f['sharedRule']=True;used.add(i)
 for f in fields:
  if f['kind'] not in ['name','type','stats']:continue
  secondary=case['layout'] in ['adventure','prepare'] and f['face']==1
  candidates=[]
  for i,l in enumerate(rows):
   if i in used:continue
   if case['layout']=='split' and f['kind'] in ['name','type'] and ((f['face']==0 and l['x']>.5) or (f['face']==1 and l['x']<.5)):continue
   if f['kind']=='name' and not (l['y']>.5 and (l['x']>.5 if case['layout']=='prepare' else l['x']<.5) if secondary else l['y']<.25):continue
   if f['kind']=='type' and not (.1 if case['layout']=='flip-face' else .2)<l['y']<.9:continue
   if f['kind']=='stats' and (l['y']<(.1 if case['layout']=='flip-face' else .6 if case['layout']=='leveler' else .75) or l['x']<.7):continue
   if f['kind']=='stats' and printing.get('set')=='mp2':continue
   aliases=[f['text']]
   if f['kind']=='type':
    text=f['text'];aliases += [re.sub(r'^Creature — ', 'Summon ',text),re.sub(r'^World Enchantment$','Enchant World',text),re.sub(r'^Enchantment — Aura$','Enchant Creature',text),re.sub(r'^Instant$','Interrupt',text)]
   score=max(similarity(l['text'],text) for text in aliases)
   if f['kind']=='type' and printing.get('frame') in ['1993','1997']:
    legacy=re.fullmatch(r'Legend|Sum+on [A-Za-z -]+|Enchant (?:Creature|Land|World|Artifact)',l['text'],re.I)
    summon=.45<l['y']<.65 and similarity(l['text'].split()[0],'Summon')>.75
    if legacy or summon:score=max(score,.9)
    if summon:l['text']=re.sub(r'^\S+','Summon',l['text'])

   if case['layout']=='split' and f['kind']=='type' and 'Room' in f['text'] and not re.search(r'^(?:enchantment.*\b[rk]oom|[rk]oom)$',l['text'],re.I):score=0
   if f['kind']=='name':
    score=max(score,registration.similarity(f['text'],l['text']))
    if l['y']<.12 and score>.7:score+=.05
    face_data=(printing.get('card_faces') or [printing])[f['face'] if printing.get('card_faces') else 0]
    alias=face_data.get('flavor_name')
    if alias:score=max(score,registration.similarity(alias,l['text'])+.25 if similarity(alias,l['text'])>.65 else 0)

   candidates.append((score,i))
  if candidates:
   score,i=max(candidates,key=lambda item:(item[0],-rows[item[1]]['y'])) if f['kind']=='stats' and case['layout']=='leveler' else max(candidates)
   if score>=.58:
    f['lines']=[rows[i]];used.add(i)
    if f['kind']=='name' and face_data.get('flavor_name'):
     f['printedText']=face_data['flavor_name'];f['headingAlias']=True
    if f['kind']=='name':
     # Vision may split a long title into disjoint fragments on one baseline.
     # Register every title fragment, otherwise the first word survives masking.
     target=normalized(f.get('printedText',f['text']))
     for j,l in enumerate(rows):
      segment=normalized(l['text'])
      if j not in used and abs(l['y']-rows[i]['y'])<.012 and len(segment)>3 and segment in target and l['x']+l['width']<.88:
       f['lines'].append(l);used.add(j)
    if f['kind']=='type':
     for j,l in enumerate(rows):
      if case['layout']=='split' and (l['x']>.5)!=(rows[i]['x']>.5):continue
      if j not in used and abs(l['y']-rows[i]['y'])<.012 and registration.similarity(l['text'],f['text'])>.7 and len(normalized(l['text']))>3:
       f['lines'].append(l);used.add(j)
 if printing.get('set')=='mp2':
  for f in fields:
   if f['kind']!='stats' or f['lines']:continue
   power,toughness=f['text'].split('/')
   candidates=[(i,l) for i,l in enumerate(rows) if i not in used and l['x']>.82 and l['y']>.76 and l['text'].strip() in [power,toughness]]
   candidates.sort(key=lambda item:item[1]['y'])
   pairs=[(a,b) for a in candidates for b in candidates if a[1]['text'].strip()==power and b[1]['text'].strip()==toughness and .025<b[1]['y']-a[1]['y']<.12]
   if pairs:
    a,b=min(pairs,key=lambda pair:pair[0][1]['y']);f['lines']=[a[1],b[1]];f['stackedStats']=True;used.update([a[0],b[0]])
 for f in list(fields):
  if not f.get('headingAlias'):continue
  candidates=[(i,l) for i,l in enumerate(rows) if i not in used and l['y']<.25 and similarity(l['text'],f['text'])>.85]
  if candidates:
   i,l=max(candidates,key=lambda item:similarity(item[1]['text'],f['text']));used.add(i)
   fields.append(dict(kind='subtitle',text=f['text'],face=f['face'],lines=[l]))
 if case['layout']=='split':
  for f in fields:
   if f['kind']!='rule' or not (f['text'].startswith('(') and f['text'].endswith(')')):continue
   existing=next((g for g in fields if g.get('sharedRule') and g['text']==f['text']),None)
   if existing:f['sharedDuplicate']=True;continue
   matching=[(i,l) for i,l in enumerate(rows) if i not in used and l['width']>.5 and registration.similarity(l['text'],f['text'])>.8]
   if matching:f['lines']=[l for i,l in matching];f['sharedRule']=True;used.update(i for i,l in matching)
 # Loyalty badges are independent frame rails, not words in the paragraph.
 for f in list(fields):
  face_data=(printing.get('card_faces') or [printing])[f['face'] if printing.get('card_faces') else 0]
  cost=re.match(r'^([+−-]?\d+):\s*',f['text']) if f['kind']=='rule' and 'Planeswalker' in face_data.get('type_line','') else None
  if not cost:continue
  f['loyaltyCost']=cost[1]
  # OCR can merge a shield and its neighboring prose. Separate their native
  # rails before paragraph alignment, so clearing prose never erases a shield.
  for l in rows:
   prefix=re.match(r'^([+−-]?\d+)\s*:\s*(\S.*)',l['text']) if l['x']<.14 and l['y']>.5 else None
   if not prefix or normalized(prefix[1])!=normalized(cost[1]):continue
   body_x=min((q['x'] for q in rows if .13<q['x']<.25 and q['y']>.5 and len(q['text'].split())>3),default=.17)
   if body_x<=l['x']+.04:
    # Retro walkers print the cost inline, with no independent shield rail.
    f.pop('loyaltyCost',None);f['inlineLoyalty']=True;continue
   badge_width=min(.075,body_x-l['x']-.025)
   rows.append({**l,'text':prefix[1],'width':badge_width})
   right=l['x']+l['width'];l.update(text=prefix[2],x=body_x,width=right-body_x)
  candidates=[(i,l) for i,l in enumerate(rows) if i not in used and l['x']<.2 and l['y']>.5 and normalized(l['text'])==normalized(cost[1])]
  if not candidates:continue
  i,l=min(candidates,key=lambda item:item[1]['y']);used.add(i)
  fields.append(dict(kind='loyalty-cost',text=cost[1],face=f['face'],ruleIndex=f['index'],lines=[l],polarity='light'))
 if case['layout']=='leveler':
  # Tier labels and tier P/T are frame rails, not paragraphs. Keep each rail
  # independent of the abilities beside it and of the base creature stats.
  for f in fields:
   if f['kind']!='rule':continue
   if re.fullmatch(r'[0-9*]+/[0-9*]+',f['text']):
    candidates=[(i,l) for i,l in enumerate(rows) if i not in used and l['x']>.7 and l['y']>.69 and normalized(l['text'])==normalized(f['text'])]
    if candidates:
     i,l=min(candidates,key=lambda item:item[1]['y']);f['lines']=[l];used.add(i);f['kind']='tier-stats'
   elif f['text'].startswith('LEVEL '):
    target=normalized(f['text'][6:]);candidates=[(i,l) for i,l in enumerate(rows) if i not in used and l['x']<.25 and l['y']>.69 and normalized(l['text'])==target]
    if candidates:
     i,l=min(candidates,key=lambda item:item[1]['y']);f['lines']=[l];used.add(i)
     for j,other in enumerate(rows):
      if j not in used and other['x']<.25 and similarity(other['text'],'LEVEL')>.65 and 0<=l['y']-other['y']<.05:f['lines'].insert(0,other);used.add(j)
     f['kind']='level-marker'
 for face in {f['face'] for f in fields}:
  ff=[f for f in fields if f['face']==face]
  types=[l for f in ff if f['kind']=='type' for l in f['lines']]
  if not types and case['layout']=='split':types=[l for f in fields if f['kind']=='type' for l in f['lines']]
  if not types:continue
  ty=types[0]['y'];bottom_type=ty>.8 or case['layout']=='saga' and ty>.65 or case['layout']=='flip-face' and ty>.22
  artist=normalized(printing.get('artist',''))
  footer=[l['y'] for l in rows if l['y']>.85 and (re.search(r'^ill[ul]*[auo]?s|©|™|wizards of the coast',l['text'],re.I) or l['y']>.88 and re.search(r'^\d+/\d+\b|\bMPS[* ]',l['text']) or artist and (artist in normalized(l['text']) or similarity(l['text'],artist)>.72))]
  if case['layout']=='modal_dfc' and case.get('face') is not None:
   other=(printing.get('card_faces') or [])[1-case['face']]
   other_types=normalized(other.get('type_line','')).split()
   other_names=[word for word in normalized(other.get('name','')).split() if len(word)>=4]
   footer += [l['y'] for l in rows if l['y']>.87 and l['x']<.3 and any(word in normalized(l['text']) for word in other_types+other_names if len(word)>=3)]
  body_bottom=min(footer+[.915])
  body=[]
  for i,l in enumerate(rows):
   if i in used or l['y']<.1 or l['y']>=body_bottom:continue
   # After normalization the collector footer remains vertical; a narrow OCR
   # fragment such as its year must never become a horizontal ability row.
   if l['width']<l['height']*.6:continue
   if case['layout']=='modal_dfc' and l['y']>.88 and re.fullmatch(r'\d+',l['text'].strip()):continue
   if l['y']>.85 and re.search(r'^ill[ul]*[auo]?s[.,:]?\s|©|™|wizards of the coast|^\d+[/]\d+\s+[RUMC]$',l['text'],re.I):continue
   if bottom_type:
    saga_creature=case['layout']=='saga' and 'Creature' in faces[face].get('type_line','')
    if l['y']>=ty and not (saga_creature and l['y']>ty+types[0]['height']):continue
    if case['layout'] in ['class','case'] and l['x']<.45:continue
    if case['layout']=='saga' and (l['x']>.65 or l['x']<.105 and re.fullmatch(r'[ivx1l ]{1,5}',normalized(l['text']))):continue
   elif l['y']<ty+types[0]['height']:continue
   if case['layout'] in ['adventure','prepare']:
    left_face=0 if case['layout']=='prepare' else 1
    if (face==left_face and l['x']>.5) or (face!=left_face and l['x']<.5):continue
   if case['layout']=='split' and ((face==0 and l['x']>.5) or (face==1 and l['x']<.5)):continue
   body.append((i,l))
  if case['layout']=='split':
   for f in ff:
    if f['kind']=='rule' and f['text'].startswith('(') and f['text'].endswith(')'):
     anchor_end=max((l['y']+l['height'] for l in f['lines']),default=0)
     for i,l in body:
      if registration.similarity(l['text'],f['text'])>=.8 and (l['width']>.5 or anchor_end and l['y']<anchor_end+.035):f['lines'].append(l);used.add(i)
   body=[(i,l) for i,l in body if i not in used]
  body.sort(key=lambda pair:(round(pair[1]['y']/.012),pair[1]['x']))
  flowing=sorted([f for f in ff if f['kind']=='rule' and not f.get('sharedDuplicate') and not (case['layout']=='split' and f['lines'])],key=lambda f:f['index'])+[f for f in ff if f['kind']=='flavor']
  if not flowing:continue
  # Monotone paragraph alignment avoids assigning a repeated phrase in a later
  # ability to an earlier one. Unrelated labels and art lettering may be skipped.
  states={-1:(0,[])}
  for i,l in body:
   scores=[]
   for f in flowing:
    text=f['text']
    face_data=(printing.get('card_faces') or [printing])[face if printing.get('card_faces') else 0]
    named=re.sub(r'this (?:creature|artifact|enchantment|Saga|token|Class|Case|Vehicle|battle|Siege|land)',face_data['name'],text,flags=re.I)
    historical=re.sub(r' to your mana pool|one colorless mana','',l['text'],flags=re.I)
    # Costs/pips are independently drawn glyphs and OCR commonly reads them as
    # letters. Match the effect after the colon without trusting those glyphs.
    effect=historical.split(':',1)[-1] if ':' in historical and ':' in text else historical
    oracle_effect=text.split(':',1)[-1] if ':' in text else text
    named=re.sub(r'this land',face_data['name'],named,flags=re.I)
    legacy_name=re.sub(re.escape(face_data['name']),face_data['name'].split(',')[0],l['text'],flags=re.I)
    score=max(registration.similarity(legacy_name,text),registration.similarity(text,legacy_name),registration.similarity(l['text'],text),registration.similarity(l['text'],named),registration.similarity(effect,oracle_effect),registration.similarity(text,l['text']),registration.similarity(named,l['text']))
    if re.match(r'^\{T\}: Add (?:\{[^}]+\}|[,or\s])+\.$',text) and re.search(r'(?<![a-z])Add(?![a-z])',l['text'],re.I):score=max(score,.9)
    # A standalone keyword must bind to its own paragraph rather than the
    # longer later paragraph that repeats it (e.g. tokens with lifelink).
    exact=[j for j,g in enumerate(flowing) if normalized(l['text'])==normalized(re.sub(r'\([^)]*\)','',g['text']))]
    if exact and flowing.index(f) not in exact:score=0
    prefix=text.split('(',1)[0].strip()
    if '(' in text and len(normalized(prefix).split())<=2 and any(normalized(row['text'])==normalized(prefix) for _,row in body):
     if normalized(l['text'])!=normalized(prefix) and not any('(' in row['text'] and abs(row['y']-l['y'])<.05 for _,row in body):score=0
    if case['layout']=='class' and re.search(r'Level [23]',l['text']):
     score=1 if re.search(r'Level [23]',l['text']).group() in text else 0
    if f.get('loyaltyCost'):
     badges=sorted([g for g in ff if g['kind']=='loyalty-cost'],key=lambda g:g['ruleIndex'])
     current=next((g for g in badges if g['ruleIndex']==f['index']),None);later=[g for g in badges if g['ruleIndex']>f['index']]
     if current and (l['y']<current['lines'][0]['y']-.025 or later and l['y']>=later[0]['lines'][0]['y']-.025):score=0
    if case['layout']=='leveler' and f['kind']=='rule':
     markers=sorted([g for g in ff if g['kind']=='level-marker' and g['lines']],key=lambda g:g['index'])
     previous=[g for g in markers if g['index']<f['index']]
     following=[g for g in markers if g['index']>f['index']]
     def band_start(marker):
      stat=next((g for g in ff if g['kind']=='tier-stats' and g['index']==marker['index']+1 and g['lines']),None)
      return min(l['y'] for g in [marker]+([stat] if stat else []) for l in g['lines'])-.025
     start=band_start(previous[-1]) if previous else ty
     end=band_start(following[0]) if following else body_bottom
     if not start<=l['y']<end:score=0
    scores.append(score)
   # A printed modal bullet starts a new option. Its continuation lines may
   # share wording, but the next bullet cannot belong to the previous option.
   if re.match(r'^[•—–-]\s*Destroy\b',l['text'],re.I):
    target=max(range(len(flowing)),key=lambda j:similarity(l['text'],flowing[j]['text']))
    scores=[score if j==target else 0 for j,score in enumerate(scores)]
   next_states={k:(v[0],v[1]+[(i,None)]) for k,v in states.items()}
   for k,(total,path) in states.items():
    for j in range(max(0,k),len(flowing)):
     score=scores[j]
     if j==k and re.match(r'^[•—–-]\s*Destroy\b',l['text'],re.I) and any(old_j==j and re.match(r'^[•—–-]\s*Destroy\b',rows[old_i]['text'],re.I) for old_i,old_j in path):continue
     if j==k and re.search(r':\s*Add\b',l['text']) and re.match(r'^\{[^}]+\}.*: Add\b',flowing[j]['text']) and any(old_j==j and re.search(r':\s*Add\b',rows[old_i]['text']) for old_i,old_j in path):continue
     if score<.52:continue
     reward=(min(score,1)-.48)*max(1,min(8,len(normalized(l['text']).split())))
     value=total+reward
     if j not in next_states or value>next_states[j][0]:next_states[j]=(value,path+[(i,j)])
   states=next_states
  _,path=max(states.values(),key=lambda v:v[0])
  for i,j in path:
   if j is not None:flowing[j]['lines'].append(rows[i]);used.add(i)
  anchored=[f for f in flowing if f['lines']]
  # Canonical wording can omit an old reminder or a full legendary name.
  # Once a paragraph is anchored, capture intervening/adjacent printed rows
  # spatially so their lettering cannot survive behind its replacement.
  for i,l in body:
   if i in used or len(normalized(l['text']))<4 or not anchored:continue
   def distance(f):
    top=min(r['y'] for r in f['lines']);end=max(r['y']+r['height'] for r in f['lines'])
    return max(top-l['y']-l['height'],l['y']-end,0)
   nearest=min(anchored,key=distance)
   if distance(nearest)<=.05:
    nearest['lines'].append(l);used.add(i)
 # Class level costs are separate OCR segments on the same baseline as
 # 'Level N'. Keep both segments in that field, never in the preceding ability.
 if case['layout']=='class':
  for f in fields:
   if f['kind']!='rule' or not re.search(r': Level [23]$',f['text']) or not f['lines']:continue
   row=next((l for l in f['lines'] if 'Level' in l['text']),f['lines'][0])
   segments=[l for l in rows if abs(l['y']-row['y'])<.012 and .45<l['x']<.93]
   f['labelParts']={'left':f['text'].split(':',1)[0]+':','right':re.search(r'Level [23]',f['text']).group()}
   f['fontLines']=[l for l in segments if 'Level' in l['text']]
   for other in fields:
    other['lines']=[l for l in other['lines'] if l not in segments]
   f['lines']=segments
 if case['layout']=='prototype':
  for f in list(fields):
   if f['kind']!='rule' or not f['text'].startswith('Prototype '):continue
   f['prototypeRail']=True
   for l in list(f['lines']):
    if l['x']>.75 and re.fullmatch(r'[0-9*]+/[0-9*]+',l['text']):
     f['lines'].remove(l);fields.append(dict(kind='preview-stats',text=l['text'],face=f['face'],lines=[l]))
 for f in fields:
  f['lines']=[{k:l[k] for k in ['text','x','y','width','height']} for l in f['lines']]
 if printing.get('_reviewedFields'):fields=printing['_reviewedFields']
 return dict(id=case['id'],face=case.get('face'),source=case['source'],layout=case['layout'],set=case['set'],fields=fields)

def build(case,printing,observations,reverse_observations=None):
 if case['layout']=='split' and any('Aftermath' in f.get('oracle_text','') for f in printing.get('card_faces',[])):
  halves=[]
  for face,face_data in enumerate(printing['card_faces']):
   vertical=face==1
   rows=[l for l in observations if l['height']>l['width'] and l['y']>.5] if vertical else [l for l in observations if l['height']<l['width'] and l['y']<.54]
   if vertical:rows=[{**l,'x':l['y'],'y':1-l['x']-l['width'],'width':l['height'],'height':l['width']} for l in rows]
   r,reason=build({**case,'layout':'aftermath-face','face':face},printing,rows)
   if not r:return None,'aftermath-'+reason
   for f in r['fields']:
    f['noFlow']=True
    if vertical:
     f['scanAspect']=488/680;f['rotation']=90
     f['orientedGeometry']={k:f[k] for k in ['bounds','lines','region'] if k in f}
     def native(b):return {**b,'x':1-b['y']-b['height'],'y':b['x'],'width':b['height'],'height':b['width']}
     for k in ['bounds','region']:
      if k in f:f[k]=native(f[k])
     f['lines']=[native(l) for l in f['lines']]
    elif 'region' in f:
     f['region']['height']=min(f['region']['height'],.525-f['region']['y'])
   halves+=r['fields']
  return {**r,'face':case.get('face'),'layout':'split','fields':halves},None
 if case['layout']=='flip':
  if reverse_observations is None:return None,'missing-rotated-ocr'
  halves=[]
  for face,ocr in enumerate([observations,reverse_observations]):
   r,reason=build({**case,'face':face,'layout':'flip-face'},printing,ocr)
   if not r:return None,'flip-'+reason
   for f in r['fields']:
    f['noFlow']=True
    f['polarity']='light' if f['kind']=='name' and any(g['kind']=='type' and g.get('bounds',{}).get('y',1)<.2 for g in r['fields']) else 'dark'
    if face==0 and f['kind']=='name':f['limit']=min(f.get('limit',1),.78)
    if face==1:
     f['rotation']=180
     def reverse(b):return {**b,'x':1-b['x']-b['width'],'y':1-b['y']-b['height']}
     for key in ['bounds','region']:
      if key in f:f[key]=reverse(f[key])
     for key in ['lines','fontLines']:
      if key in f:f[key]=[reverse(b) for b in f[key]]
   halves.extend(r['fields'])
  return {**r,'face':case.get('face'),'layout':'flip','fields':halves},None

 split_rotated=case['layout']=='split' and all(any(l['height']>l['width'] and registration.similarity(f['name'],l['text'])>.65 for l in observations) for f in printing.get('card_faces',[]))
 if case['layout']=='split' and not split_rotated:return None,'rotated-or-combined'
 face_data=(printing.get('card_faces') or [printing])[case.get('face') or 0]
 source_layout=case['layout']
 if 'Saga' in face_data.get('type_line',''):case={**case,'layout':'saga'}
 rotation=90 if split_rotated or 'Battle' in face_data.get('type_line','') else 0
 if rotation:
  observations=[{**l,'x':1-l['y']-l['height'],'y':l['x'],'width':l['height'],'height':l['width']} for l in observations]
  # Portrait footer text becomes vertical after normalizing a landscape face.
  observations=[l for l in observations if l['height']<l['width'] or re.fullmatch(r'[0-9*/+–-]+',l['text'])]

 opaque=printing.get('set')=='mp2'
 if opaque:
  # Decorative letterforms have no matching font. Preserve them when values
  # agree; an actual changed header reconstructs this bounded label interior.
  observations += [dict(text=face_data['name'],confidence=1,x=.075,y=.06,width=.77,height=.04),dict(text=face_data['type_line'],confidence=1,x=.075,y=.575,width=.75,height=.04)]
 previews=[]
 if rotation and 'Battle' in face_data.get('type_line','') and printing.get('card_faces'):
  back=printing['card_faces'][1-(case.get('face') or 0)]
  stat=f"{back['power']}/{back['toughness']}" if back.get('power') is not None else None
  if stat:
   isolated=[line for line in observations if line['text'].strip()==stat and line['x']>.85]
   for line in isolated:previews.append(dict(kind='preview-stats',text=stat,face=case.get('face') or 0,lines=[line]))
   observations=[line for line in observations if line not in isolated]
   for i,line in enumerate(observations):
    if line['text'].endswith(' '+stat) and line['x']<.3 and line['x']+line['width']>.9:
     boundary=.868
     previews.append(dict(kind='preview-stats',text=stat,face=case.get('face') or 0,lines=[{**line,'text':stat,'x':boundary,'width':line['x']+line['width']-boundary}]))
     observations[i]={**line,'text':line['text'][:-len(stat)].strip(),'width':boundary-line['x']}
 r=contextual_registration(case,printing,observations)
 r['fields'].extend(previews)
 if opaque:
  for f in r['fields']:
   if f['kind'] in ['name','type']:f['opaqueHeader']=True;f['fontSizeHint']=.042 if f['kind']=='name' else .034
   elif f['kind']=='rule':
    f['rebuildPrintedLines']=True
    f['fontFamily']='Arial, sans-serif'
    f['polarity']='dark'
    if f['lines']:f['fontLines']=[max(f['lines'],key=lambda l:l['width'])]
   elif f['kind']=='stats':f['polarity']='dark'
 r['collector_number']=case['number'];r['lang']=printing.get('lang','en')
 if printing.get('textless') and 'showcase' in printing.get('frame_effects',[]):
  for f in r['fields']:
   if f['kind'] in ['name','type']:f['opaqueHeader']=True
 if rotation:
  r['rotation']=rotation
  for f in r['fields']:
   f['scanAspect']=488/680
   if f['kind']=='stats':f['polarity']='light'
 for field in r['fields']:
  # The base OCR generator used the selected-face array index for stats.
  if case.get('face') is not None:field['face']=case['face']
  lines=field['lines']
  if field['kind'] in ['name','type'] and not field.get('reviewedRegion'):
   secondary=case['layout'] in ['adventure','prepare'] and field['face']==1
   if field['kind']=='name':lines=[l for l in lines if (l['y']>.5 if secondary else l['y']<.25)]
   elif case['layout'] in ['saga','class','case'] and any(l['y']>.7 for l in lines):lines=[l for l in lines if l['y']>.7]
   else:lines=[l for l in lines if (.1 if case['layout']=='flip-face' else .2)<l['y']<.9]
   if lines:
    best=max(lines,key=lambda l:registration.similarity(l['text'],field['text']))
    lines=[l for l in lines if abs(l['y']-best['y'])<.025]
  if field['kind']=='type':
   indicator=(printing.get('card_faces') or [printing])[field['face'] if printing.get('card_faces') else 0].get('color_indicator')
   if indicator:
    lines=[{**l,'text':re.sub(r'^(?:[©°○●]|O(?=\s))\s*','',l['text']),'x':.125,'width':l['width']-(.125-l['x'])} if l['x']<.1 and re.match(r'^(?:[©°○●]|O\s)',l['text']) else l for l in lines]
  if field['kind']=='name':
   trimmed=[]
   for l in lines:
    prefix=re.match(r'^[)©°○●]\s+',l['text'])
    if prefix:
     shift=l['height']*field.get('scanAspect',680/488)
     l={**l,'text':l['text'][prefix.end():],'x':l['x']+shift,'width':l['width']-shift}
    trimmed.append(l)
   lines=trimmed
  if field['kind']=='stats' and not field.get('stackedStats'):lines=[l for l in lines if similarity(l['text'],field['text'])>.7]
  field['lines']=sorted(lines,key=lambda l:(round(l['y']/.025) if field['kind'] in ['name','type'] else l['y'],l['x']))
  if lines:field['bounds']=bounds(lines)
  else:field.pop('bounds',None)
 # Only full registrations may enter the runtime catalog.
 r['fields']=[f for f in r['fields'] if not f.get('sharedDuplicate') and not (case['layout']=='split' and f['kind']=='type' and not f.get('bounds') and any(g['kind']=='type' and g.get('bounds') and g['text']==f['text'] for g in r['fields']))]
 r['fields']=[f for f in r['fields'] if f.get('bounds') or not (f['kind']=='rule' and (f['text'].startswith('(') and f['text'].endswith(')') or re.fullmatch(r'Enchant \w+',f['text']) and any(g['kind']=='type' and normalized(f['text']) in normalized(' '.join(l['text'] for l in g['lines'])) for g in r['fields'])))]
 if any(not f.get('bounds') for f in r['fields'] if f['kind']!='flavor'):return None,'missing-field'
 fields=[f for f in r['fields'] if f.get('bounds')]
 for f in fields:
  if printing.get('set')=='mps' and f['kind'] in ['name','type']:f['polarity']='dark'
  if f['kind'] in ['rule','type']:f['printedText']=f.get('printedTextOverride') or ' '.join(l['text'] for l in f['lines'])
 for f in fields:
  b=f['bounds']
  if b['x']<0 or b['y']<0 or b['x']+b['width']>1.001 or b['y']+b['height']>(.97 if rotation else .95):return None,'outside-print'
  if f['kind'] in ['name','type'] and not (f.get('opaqueHeader') or f.get('opaqueLettering')) and (b['height']>b['width'] or b['height']>.09):return None,'header-geometry'
  if f['kind']=='rule':
   if f.get('reviewedPrintedText'):continue
   printed=' '.join(l['text'] for l in f['lines'])
   short=re.sub(r'\([^)]*\)','',f['text'])
   historical=re.sub(r' to your mana pool|one colorless mana','',printed,flags=re.I)
   effect=historical.split(':',1)[-1] if ':' in historical and ':' in short else historical
   oracle_effect=short.split(':',1)[-1] if ':' in short else short
   face_data=(printing.get('card_faces') or [printing])[f['face'] if printing.get('card_faces') else 0]
   named=re.sub(r'this (?:creature|artifact|enchantment|land|Saga|Class|Case|battle)',face_data['name'],short,flags=re.I)
   short_name=face_data['name'].split(',')[0]
   legacy_name=re.sub(re.escape(face_data['name']),short_name,printed,flags=re.I)
   def historical_normalized(text):
    text=re.sub(re.escape(face_data['name']), 'this creature',text,flags=re.I)
    text=re.sub(r'converted mana cost','mana value',text,flags=re.I)
    text=re.sub(r'enters the battlefield','enters',text,flags=re.I)
    text=re.sub(r'(?:put|puts) the top (\w+) cards of your library into your graveyard',r'mill \1 cards',text,flags=re.I)
    return text
   stop={'the','a','an','by','their','that','each','whenever','this','it','is','of','to','and'}
   a=set(normalized(printed).split())-stop;b=set(normalized(short).split())-stop
   vocabulary=2*len(a&b)/(len(a)+len(b)) if len(a&b)>=3 else 0
   vocabulary=vocabulary if vocabulary>=.7 else 0
   if max(vocabulary,similarity(historical_normalized(printed),historical_normalized(short)),similarity(legacy_name,short),similarity(printed,f['text']),similarity(printed,short),similarity(effect,oracle_effect),similarity(printed,named))<.48:return None,'text-coverage'
 for i,a in enumerate(fields):
  for b in fields[i+1:]:
   if a['face']!=b['face']:continue
   # Paragraph envelopes include empty space beside the final line (often
   # occupied by P/T). Only actual OCR line intersections are ambiguous.
   for la in a['lines']:
    for lb in b['lines']:
     area=overlap(la,lb)
     if a.get('opaqueLettering') and b.get('opaqueLettering'):continue
     if area>min(la['width']*la['height'],lb['width']*lb['height'])*.25:return None,'overlapping-fields'
 for f in fields:
  if f.get('sharedRule'):
   f['noFlow']=True;f['rebuildPrintedLines']=True
   if f['text'].startswith('('):f['polarity']='light';f['outlined']=False
   elif f['text'].startswith('Fuse '):f['polarity']='dark';f['outlined']=False
   b=f['bounds'];f['region']={**b,'y':b['y']-.003,'height':b['height']+.012}
  if f.get('opaqueLettering'):f['protectedBounds']=[g['bounds'] for g in fields if g is not f and g.get('opaqueLettering') and overlap(f['bounds'],g['bounds'])>0]
 # Preserve segmented columns when replacement text grows. OCR supplies the
 # ink; layout metadata supplies the panel's available space.
 for face in {f['face'] for f in fields}:
  flowing=[f for f in fields if f['face']==face and f['kind'] in ['rule','flavor'] and not f.get('reviewedRegion') and not f.get('sharedRule')]
  saga_type=next((f['bounds'] for f in fields if f['face']==face and f['kind']=='type'),None)
  if case['layout']=='saga' and 'Creature' in (printing.get('card_faces') or [printing])[face if printing.get('card_faces') else 0].get('type_line','') and saga_type:
   for f in list(flowing):
    b=f['bounds']
    if b['y']>saga_type['y'] or f['text'].startswith('(') and b['width']>.5:
     f['noFlow']=True;f['region']={**b,'y':b['y']-.003,'height':b['height']+.015};flowing.remove(f)
  if not flowing:continue
  x=min(f['bounds']['x'] for f in flowing);right=max(f['bounds']['x']+f['bounds']['width'] for f in flowing)
  if case['layout'] in ['class','case'] and saga_type and saga_type['y']>.8:x=max(x,.51)
  type_inset=next((f['bounds']['x'] for f in fields if f['face']==face and f['kind']=='type'),None)
  if type_inset is not None and 0<type_inset<.25 and all(abs(l['x']+l['width']/2-.5)<.025 for f in flowing for l in f['lines']):x=min(x,type_inset);right=max(right,1-type_inset)
  top=min(f['bounds']['y'] for f in flowing)-.003
  stats=[f['bounds']['y'] for f in fields if f['face']==face and f['kind']=='stats' and f['bounds']['y']>top]
  artist=normalized(printing.get('artist',''))
  footers=[l['y'] for l in observations if l['y']>.85 and (re.search(r'^ill[ul]*[auo]?s|©|™|wizards of the coast',l['text'],re.I) or artist and (artist in normalized(l['text']) or similarity(l['text'],artist)>.72))]
  if case['layout']=='modal_dfc' and case.get('face') is not None:
   other_types=normalized(printing['card_faces'][1-case['face']].get('type_line','')).split()
   other_names=[word for word in normalized(printing['card_faces'][1-case['face']].get('name','')).split() if len(word)>=4]
   footers += [l['y'] for l in observations if l['y']>.87 and l['x']<.3 and any(word in normalized(l['text']) for word in other_types+other_names if len(word)>=3)]
  bottom_types=[f['bounds']['y'] for f in fields if f['face']==face and f['kind']=='type' and (f['bounds']['y']>.8 or case['layout']=='saga' and f['bounds']['y']>.65 or case['layout']=='flip-face' and f['bounds']['y']>.22)]
  bottom=min(stats+bottom_types+footers+[.925])- .006
  if not bottom_types and not footers:bottom=max(bottom,max(f['bounds']['y']+f['bounds']['height'] for f in flowing)+.003)
  if case['layout']=='split':right=.485 if face==0 else .925
  elif case['layout'] in ['adventure','prepare']:
   right=.48 if face==(0 if case['layout']=='prepare' else 1) else .925
  elif case['layout']=='saga' and bottom_types:right=min(.5,right+.012)
  else:right=max(right,min(.94,1-x))
  for f in flowing:f['region']=dict(x=max(0,x-.003),y=max(0,top),width=right-x+.003,height=bottom-top)
  if case['layout']=='split':
   for f in flowing:
    if f['bounds']['width']>.5:
     b=f['bounds'];f['noFlow']=True;f['region']={**b,'y':b['y']-.003,'height':b['height']+.012}
  # Rules in separate bands retain their panel boundaries as live text grows.
  planeswalker='Planeswalker' in (printing.get('card_faces') or [printing])[face if printing.get('card_faces') else 0].get('type_line','')
  if case['layout'] in ['class','case','prototype','leveler','flip-face'] or planeswalker:
   ordered=sorted(flowing,key=lambda f:f['bounds']['y'])
   for i,f in enumerate(ordered):
    b=f['bounds'];end=ordered[i+1]['bounds']['y']-.006 if i+1<len(ordered) else bottom
    f['noFlow']=True
    panel_x=b['x']-.003 if case['layout']=='leveler' else x-.003
    panel_right=.75 if case['layout']=='leveler' else right
    if case['layout']=='prototype' and f.get('prototypeRail'):panel_right=min(panel_right,.775)
    if case['layout']=='leveler':
     markers=[g['bounds']['y']-.018 for g in fields if g['kind']=='level-marker' and g.get('bounds') and g['bounds']['y']>b['y']+b['height']]
     end=min([end]+markers)
    panel_top=max(top,b['y']-.003)
    panel_height=max(b['height']+.012,end-panel_top)
    if case['layout']=='flip-face' or planeswalker:panel_height=min(panel_height,max(.01,end-panel_top))
    f['region']=dict(x=max(0,panel_x),y=panel_top,width=panel_right-panel_x,height=panel_height)
    if case['layout'] in ['class','case'] and panel_x>=.5:
     f['protectedBounds']=f.get('protectedBounds',[])+[dict(x=0,y=0,width=.505,height=1)]
  if case['layout'] in ['adventure','prepare','split']:
   for f in fields:
    if f['face']==face and f['kind'] in ['name','type'] and (case['layout']=='split' or f['face']==(0 if case['layout']=='prepare' else 1)):
     b=f['bounds'];f['region']=dict(x=b['x'],y=b['y']-.006,width=(.485 if case['layout']=='split' and face==0 else .925 if case['layout'] in ['prepare','split'] else .48)-b['x'],height=b['height']+.015)
 r['fields']=fields
 r['layout']=source_layout
 return r,None

def main():
 p=argparse.ArgumentParser();p.add_argument('--cache',required=True,type=pathlib.Path);p.add_argument('--audit',required=True,type=pathlib.Path);p.add_argument('--only',help='Regenerate matching slugs, names or layouts while retaining other registrations');p.add_argument('--output',type=pathlib.Path,default=ROOT/'web/ui/src/lib/card-region-history.generated.js');a=p.parse_args()
 cases=json.load(open(ROOT/'web/ui/tests/frame-history-corpus.json'))['cases'];baseline={r['slug']:r for r in json.load(open(a.audit/'results.json'))};accepted=[];rejected=[]
 selected=[c for c in cases if baseline[c['slug']].get('mode')=='custom' and (not a.only or re.search(a.only,c['slug']+' '+c['name']+' '+c['layout']))]
 if a.only:
  ids={(c['id'],c['face']) for c in selected};slugs={c['slug'] for c in selected}
  if a.output.exists():
   source=a.output.read_text();accepted=[r for r in json.loads(source[source.index('[\n'):source.rindex(';')]) if (r['id'],r['face']) not in ids]
  if (a.audit/'registration-review.json').exists():rejected=[r for r in json.load(open(a.audit/'registration-review.json'))['rejected'] if r['slug'] not in slugs]
 review_path=ROOT/'web/ui/tests/frame-history-manual-fields.json'
 reviews={r['slug']:r for r in json.load(open(review_path))} if review_path.exists() else {}
 for c in selected:
  printing=json.load(open(a.cache/(c['id']+'.json')));observations=json.load(open(a.audit/'ocr'/(c['slug']+'-original.json')))['lines']
  if c['slug'] in reviews:
   review=reviews[c['slug']]
   if hashlib.sha256((a.cache/(c['slug']+'-normal.jpg')).read_bytes()).hexdigest()!=review['sourceSha256']:raise ValueError('Reviewed scan changed: '+c['slug'])
   if review.get('fields'):printing['_reviewedFields']=review['fields']
  refined=a.audit/'ocr-extra'/(c['slug']+'-body.json')
  if refined.exists() and c['layout'] not in ['split','flip']:
   improved=json.load(open(refined))['lines']
   observations=[o for o in observations if not any(overlap(l,o)>min(l['width']*l['height'],o['width']*o['height'])*.4 for l in improved)]+improved
  extra=a.audit/'ocr-extra'/(c['slug']+'.json')
  if extra.exists():
   for l in json.load(open(extra))['lines']:
    observations=[o for o in observations if not (re.fullmatch(r'[0-9*/+–-]+',o['text']) and overlap(l,o)>min(l['width']*l['height'],o['width']*o['height'])*.5)]
    if not any(overlap(l,o)>min(l['width']*l['height'],o['width']*o['height'])*.5 for o in observations):observations.append(l)
  upright_path=a.audit/'ocr-rotated'/(c['slug']+'-0.json')
  if c['layout']=='flip' and upright_path.exists():observations=json.load(open(upright_path))['lines']
  reverse_path=a.audit/'ocr-rotated'/(c['slug']+'-180.json')
  reverse=json.load(open(reverse_path))['lines'] if c['layout']=='flip' and reverse_path.exists() else None
  r,reason=build(c,printing,observations,reverse)
  if r:
   # A visually reviewed correction may target a field after split/flip
   # coordinates have been combined. The source hash above pins the scan.
   for override in reviews.get(c['slug'],{}).get('fieldOverrides',[]):
    matches=[f for f in r['fields'] if all(f.get(k)==v for k,v in override['match'].items())]
    if len(matches)!=1:raise ValueError('Ambiguous reviewed field: '+c['slug'])
    matches[0].update(override['patch'])
   # Loyalty shields come with either pale or dark interiors. Sample the
   # registered digit's surrounding paper rather than assuming white ink.
   badges=[f for f in r['fields'] if f['kind']=='loyalty-cost']
   if badges:
    from PIL import Image
    image=Image.open(a.cache/(c['slug']+'-normal.jpg')).convert('L');w,h=image.size
    for f in badges:
     b=f['bounds'];x=int(b['x']*w);y=int(b['y']*h);right=int((b['x']+b['width'])*w);bottom=int((b['y']+b['height'])*h)
     pixels=[image.getpixel((px,py)) for py in range(max(0,y),min(h,bottom)) for px in range(max(0,x),min(w,right))]
     f['polarity']='dark' if sorted(pixels)[len(pixels)//2]>140 else 'light'
     f['outlined']=False
   accepted.append(r)
  else:rejected.append(dict(slug=c['slug'],name=c['name'],reason=reason))
 order={(c['id'],c['face']):i for i,c in enumerate(cases)};accepted.sort(key=lambda r:order[(r['id'],r['face'])])
 a.output.write_text('// Generated by scripts/register-frame-history.py from pinned metadata and local OCR.\nexport default [\n'+',\n'.join(json.dumps(r,separators=(',',':')) for r in accepted)+'\n];\n')
 (a.audit/'registration-review.json').write_text(json.dumps(dict(accepted=len(accepted),rejected=rejected),indent=2))
 print('Accepted',len(accepted),collections.Counter(r['layout'] for r in accepted),'rejected',collections.Counter(r['reason'] for r in rejected))
if __name__=='__main__':main()
