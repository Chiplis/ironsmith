"""Pixel geometry for isolated stat badges, independent of text recognition."""
import collections
from PIL import Image,ImageOps
def badge_digit_box(crop):
 # A light digit is disconnected from the light border of its dark badge.
 # Segment those components before OCR so the star border cannot become ',4'.
 pixels=crop.convert('L');w,h=pixels.size;data=pixels.load();seen=set();candidates=[]
 for y in range(h):
  for x in range(w):
   if (x,y) in seen or data[x,y]<200:continue
   queue=collections.deque([(x,y)]);seen.add((x,y));component=[]
   while queue:
    px,py=queue.popleft();component.append((px,py))
    for nx,ny in [(px-1,py),(px+1,py),(px,py-1),(px,py+1)]:
     if 0<=nx<w and 0<=ny<h and (nx,ny) not in seen and data[nx,ny]>=200:seen.add((nx,ny));queue.append((nx,ny))
   if len(component)<30:continue
   left=min(p[0] for p in component);top=min(p[1] for p in component);right=max(p[0] for p in component)+1;bottom=max(p[1] for p in component)+1
   if (right-left)*(bottom-top)>w*h*.08 or left==0 or top==0 or right==w or bottom==h:continue
   distance=((left+right)/2/w-.66)**2+((top+bottom)/2/h-.60)**2
   if distance<.03:candidates.append((distance,(max(0,left-8),max(0,top-8),min(w,right+8),min(h,bottom+8))))
 return min(candidates)[1] if candidates else None

def stat_badge_crop(image,rotated):
 if rotated:image=image.rotate(-90,expand=True)
 W,H=image.size
 box=(int(.80*W),int(.80*H),W,H) if rotated else (int(.72*W),int(.84*H),W,int(.935*H))
 crop=image.crop(box).resize(((box[2]-box[0])*5,(box[3]-box[1])*5))
 if rotated:
  digit=badge_digit_box(crop)
  if digit:
   box=(box[0]+digit[0]/5,box[1]+digit[1]/5,box[0]+digit[2]/5,box[1]+digit[3]/5)
   crop=crop.crop(digit)
  crop=ImageOps.invert(crop.convert('RGB'))
 return crop,box,(W,H)
