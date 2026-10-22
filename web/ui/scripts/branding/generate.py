#!/usr/bin/env python3
"""Generate a self-contained font-aware forging wordmark (requires fontTools)."""
import argparse
import json
import math
from pathlib import Path
import xml.etree.ElementTree as ET
from fontTools.ttLib import TTFont
from fontTools.pens.basePen import BasePen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.svgLib.path import parse_path
from fontTools.varLib.instancer import instantiateVariableFont

NS = '{http://www.w3.org/2000/svg}'
ET.register_namespace('', NS[1:-1])
TEMPLATE = Path(__file__).with_name('wordmark-template.svg')

class OutlinePen(BasePen):
    def __init__(self, glyphs=None):
        super().__init__(glyphs)
        self.contours = []
        self.points = []
    def _moveTo(self, p):
        self.points = [p]
    def _lineTo(self, p):
        self.points.append(p)
    def _curveToOne(self, a, b, c):
        p = self._getCurrentPoint()
        for i in range(1, 25):
            t = i / 24
            self.points.append(tuple((1-t)**3*p[j]+3*(1-t)**2*t*a[j]+3*(1-t)*t*t*b[j]+t**3*c[j] for j in (0, 1)))
    def _qCurveToOne(self, a, b):
        p = self._getCurrentPoint()
        for i in range(1, 25):
            t = i / 24
            self.points.append(tuple((1-t)**2*p[j]+2*(1-t)*t*a[j]+t*t*b[j] for j in (0, 1)))
    def _closePath(self):
        if len(self.points) > 2:
            self.contours.append(self.points)
        self.points = []
    _endPath = _closePath

def area(points):
    return abs(sum(x*v-u*y for (x,y),(u,v) in zip(points, points[1:]+points[:1]))) / 2

def sample(points, count=240):
    pairs = list(zip(points, points[1:]+points[:1]))
    lengths = [math.dist(a,b) for a,b in pairs]
    total = sum(lengths)
    if total <= 0:
        raise ValueError('Font contains a degenerate contour')
    result = []
    segment, start = 0, 0
    for i in range(count):
        distance = total*i/count
        while segment < len(pairs)-1 and start+lengths[segment] < distance:
            start += lengths[segment]
            segment += 1
        a,b = pairs[segment]
        t = (distance-start)/(lengths[segment] or 1)
        result.append(tuple(x+(y-x)*t for x,y in zip(a,b)))
    return result

def match(source, target):
    candidates = []
    for points in (target, list(reversed(target))):
        for i in range(len(points)):
            shifted = points[i:]+points[:i]
            cost = sum((x-u)**2+(y-v)**2 for (x,y),(u,v) in zip(source,shifted))
            candidates.append((cost, shifted))
    return min(candidates, key=lambda item:item[0])[1]

def path(points):
    return 'M'+' L'.join(f'{x:.3f} {y:.3f}' for x,y in points)+' Z'

def generate(font_path, output, axes=None, letter_spacing=0, loading_screen=False):
    font = TTFont(font_path)
    if axes:
        if 'fvar' not in font:
            raise ValueError('--axis requires a variable font')
        available = {a.axisTag for a in font['fvar'].axes}
        if set(axes)-available:
            raise ValueError('Unknown variation axis')
        font = instantiateVariableFont(font, axes, inplace=False)
    glyphs, cmap = font.getGlyphSet(), font.getBestCmap()
    missing = sorted(set('IronSmith')-set(chr(c) for c in cmap))
    if missing:
        raise ValueError('Font is missing characters: '+''.join(missing))
    word = 'IronSmit'
    spacing = letter_spacing*font['head'].unitsPerEm
    advance = sum(glyphs[cmap[ord(c)]].width+spacing for c in word)
    if advance <= 0:
        raise ValueError('Font has no usable advances')
    # Uniform scale keeps the font proportions intact. The whole word is paths,
    # so the output does not depend on installed fonts or asynchronous font loads.
    scale = min(825/advance, 330/font['head'].unitsPerEm)
    word_start = 899-advance*scale
    root = ET.parse(TEMPLATE).getroot()
    old = next(t for t in root.findall(NS+'text') if t.text == word)
    index = list(root).index(old)
    root.remove(old)
    letters = ET.Element(NS+'g', {'fill':'#fcebd0', 'class':'font-wordmark'})
    root.insert(index, letters)
    cursor = word_start
    top = 753
    for char in word:
        glyph = glyphs[cmap[ord(char)]]
        pen = SVGPathPen(glyphs)
        glyph.draw(pen)
        ET.SubElement(letters, NS+'path', {'d':pen.getCommands(), 'transform':f'matrix({scale} 0 0 {-scale} {cursor} 753)'})
        bounds = OutlinePen(glyphs)
        glyph.draw(bounds)
        top = min(top, *(753-y*scale for contour in bounds.contours for x,y in contour))
        cursor += (glyph.width+spacing)*scale
    pen = OutlinePen(glyphs)
    glyphs[cmap[ord('h')]].draw(pen)
    contours = [[((cursor-860+x*scale)/.75, 1004-y*scale/.75) for x,y in c] for c in pen.contours]
    if not contours:
        raise ValueError('Font h has no vector outline (bitmap/color-only fonts are unsupported)')
    contours.sort(key=area, reverse=True)
    outer = contours[0]
    xmin, xmax = min(x for x,y in outer), max(x for x,y in outer)
    strike_x = xmin+.68*(xmax-xmin)
    crossings = []
    for (x,y),(u,v) in zip(outer, outer[1:]+outer[:1]):
        if min(x,u) <= strike_x < max(x,u):
            crossings.append(y+(v-y)*(strike_x-x)/(u-x))
    if not crossings:
        raise ValueError('Cannot locate a strike surface on this font h')
    strike_y = min(crossings)
    anvil = root.find('.//*[@class="anvil"]')
    source_path = anvil.find(NS+'path')
    pen = OutlinePen()
    parse_path(source_path.get('d'), pen)
    # Align the starting anvil's shoulder to the font shoulder. All intermediate
    # contours keep this contact area close to the same spot as the body changes.
    width_scale = (xmax-xmin)/250*1.2
    height_scale = (1004-strike_y)/240
    if height_scale <= 0:
        raise ValueError('Font h has no usable shoulder height')
    source = [(strike_x+(x-235)*width_scale,1004+(y-1004)*height_scale) for x,y in pen.contours[0]]
    source = sample(source)
    targets = [match(source, sample(outer))]
    sources = [source]
    # Preserve every contour, including counters and disconnected ornaments.
    for contour in contours[1:]:
        target = sample(contour)
        center = tuple(sum(p[i] for p in target)/len(target) for i in (0,1))
        sources.append([center]*len(target))
        targets.append(target)
    def shape(t):
        return ' '.join(path([(x+(u-x)*t,y+(v-y)*t) for (x,y),(u,v) in zip(a,b)]) for a,b in zip(sources,targets))
    source_path.set('class','forged-letter')
    source_path.set('fill-rule','evenodd')
    source_path.set('d', shape(0))
    # Scale the complete rig about the baseline, then place its strike on the
    # font shoulder. Hands, hammer and feet keep their relative proportions.
    rig_scale = (1004-strike_y)/(1004-763.1)
    dx, dy = strike_x-210*rig_scale, 1004*(1-rig_scale)
    smith = root.find('.//*[@class="smith"]')
    smith.set('transform',f'matrix({rig_scale:.6f} 0 0 {rig_scale:.6f} {70*rig_scale+dx:.3f} {dy:.3f})')
    sparks = root.find('.//*[@class="sparks"]')
    parent = next(e for e in root.iter() if sparks in list(e))
    pos = list(parent).index(sparks)
    parent.remove(sparks)
    wrapper = ET.Element(NS+'g',{'transform':f'matrix({rig_scale:.6f} 0 0 {rig_scale:.6f} {dx:.3f} {dy:.3f})'})
    wrapper.append(sparks)
    parent.insert(pos,wrapper)
    style = root.find(NS+'style')
    style.text = style.text.replace('2s infinite','2s 3 forwards').replace('2s ease-in-out infinite','2s ease-in-out 3 forwards')
    style.text += '\n#working:target .forged-letter { animation: forge-letter 6s linear forwards; }\n@keyframes forge-letter {\n'
    for times,t in [('0%,14.66%',0),('20%,48%',.4),('54%,81.33%',.75),('88%,100%',1)]:
        style.text += f'{times} {{ d: path("{shape(t)}"); }}\n'
    style.text += '}\n@media (prefers-reduced-motion: reduce) { #working:target .forged-letter { animation:none; d:path("'+shape(1)+'"); } }\n'
    if loading_screen:
        # Transparent, tightly framed artwork for the real loading screen.
        root.remove(root.find(NS+'rect'))
        for caption in root.findall(NS+'text'):
            root.remove(caption)
        rest = root.find('.//*[@class="rest-sparks"]')
        if rest is not None:
            next(e for e in root.iter() if rest in list(e)).remove(rest)
        left = word_start-24
        top = min(top, .75*(210*rig_scale+dy))-24
        right = 860+.75*(1054*rig_scale+dx)+24
        root.set('viewBox',f'{left:.3f} {top:.3f} {right-left:.3f} {777-top:.3f}')
        root.find(NS+'title').text = 'IronSmith — forging your table'
    metadata = {'font' :Path(font_path).name,'axes':axes or {},'strike':[strike_x,strike_y], 'smithOffset':[dx,dy], 'smithScale':rig_scale, 'glyphContours':len(contours),'scale':scale, 'letterSpacing':letter_spacing, 'loadingScreen':loading_screen}
    ET.SubElement(root, NS+'metadata').text = json.dumps(metadata)
    output = Path(output)
    output.parent.mkdir(parents=True,exist_ok=True)
    output.write_text(ET.tostring(root,encoding='unicode')+'\n')
    font.close()
    return metadata

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--font',type=Path,required=True,help='TTF, OTF or WOFF outline font')
    parser.add_argument('--out',type=Path,required=True)
    parser.add_argument('--axis',action='append',default=[],metavar='TAG=VALUE')
    parser.add_argument('--letter-spacing', type=float, default=0, help='Tracking in em')
    parser.add_argument('--loading-screen', action='store_true', help='Transparent, cropped artwork without preview caption')
    args = parser.parse_args()
    try:
        axes = dict((tag,float(value)) for tag,value in (s.split('=',1) for s in args.axis))
        print(json.dumps(generate(args.font,args.out,axes,args.letter_spacing,args.loading_screen),indent=2))
    except (ValueError,KeyError,OSError) as error:
        parser.error(str(error))
