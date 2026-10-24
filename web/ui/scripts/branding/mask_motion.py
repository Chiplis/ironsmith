"""Project one rigid mask through a fixed temple hinge, viewed 30 degrees above.

The shell's 180-unit depth and camera stay constant. Only its hinge angle changes.
Run this module before generate.py when changing the hinge geometry or timing.
"""
import math
import re
from pathlib import Path
import xml.etree.ElementTree as ET

NS = '{http://www.w3.org/2000/svg}'
ELEVATION = math.radians(30)
DEPTH = 180.0
OPEN_ANGLE = 80.0
PIN = (932.0, 500.0)
AXIS = (352 / math.hypot(352, 40), 40 / math.hypot(352, 40))


def angle_at(percent):
    if percent <= 3:
        return OPEN_ANGLE
    if percent < 12:
        t = (percent - 3) / 9
        return OPEN_ANGLE * (1 - t*t*(3-2*t))
    if percent <= 86:
        return 0.0
    if percent < 98:
        t = (percent - 86) / 12
        return OPEN_ANGLE * t*t*(3-2*t)
    return OPEN_ANGLE


def rotate_point(screen_x, screen_y, angle):
    """Unproject the closed shell, then rotate in 3D around the fixed x axis."""
    ux, uy = AXIS
    dx, dy = screen_x - PIN[0], screen_y - PIN[1]
    x = dx*ux + dy*uy
    projected_y = -dx*uy + dy*ux
    y = (projected_y - DEPTH*math.sin(ELEVATION)) / math.cos(ELEVATION)
    t = math.radians(angle)
    return x, y*math.cos(t)-DEPTH*math.sin(t), y*math.sin(t)+DEPTH*math.cos(t)


def project(point):
    x, y, z = point
    sy = y*math.cos(ELEVATION) + z*math.sin(ELEVATION)
    ux, uy = AXIS
    return PIN[0]+ux*x-uy*sy, PIN[1]+uy*x+ux*sy


def matrix(angle):
    origin = project(rotate_point(0, 0, angle))
    x = project(rotate_point(1, 0, angle))
    y = project(rotate_point(0, 1, angle))
    return (x[0]-origin[0], x[1]-origin[1], y[0]-origin[0], y[1]-origin[1], *origin)


def formatted(values):
    return 'matrix(' + ','.join(f'{v:.7f}' for v in values) + ')'


def configure(root):
    pivot = root.find('.//*[@class="mask-depth-pivot"]')
    pivot.set('transform', formatted(matrix(OPEN_ANGLE)))
    # At 0–80 degrees this camera always sees the outside. Never mirror the visor.
    inside = pivot.find('./*[@class="mask-inner-face"]')
    if inside is not None:
        pivot.remove(inside)
    bracket = root.find('.//*[@class="mask-depth-bracket"]')
    x, y = project(rotate_point(*PIN, OPEN_ANGLE))
    bracket.set('d', f'M932 500 L{x:.5f} {y:.5f}')
    style = root.find(NS+'style')
    css = style.text
    for key in ['mask-depth-turn', 'mask-bracket-turn', 'mask-outer-visibility', 'mask-inner-visibility']:
        css = re.sub(r'@keyframes '+key+r' \{.*?\n\}', '', css, flags=re.S)
    css = re.sub(r'#working:target \.mask-(?:outer|inner)-face\{[^}]*\}', '', css)
    css = re.sub(r'\.mask-(?:outer|inner)-face\{opacity:[01];\}', '', css)
    css += '\n.mask-outer-face {opacity:1;}\n'
    for key in ['mask-depth-turn', 'mask-bracket-turn']:
        css += '@keyframes '+key+' {\n'
        for step in range(201):
            percent = step/2
            angle = angle_at(percent)
            if key == 'mask-depth-turn':
                rule = 'transform:'+formatted(matrix(angle))+';'
            else:
                x, y = project(rotate_point(*PIN, angle))
                rule = f'd:path("M932 500 L{x:.5f} {y:.5f}");'
            css += f'{percent:g}% {{{rule}}}\n'
        css += '}\n'
    css = '\n'.join(line for line in css.splitlines() if not line.endswith(',}'))
    css += '\n@media(prefers-reduced-motion:reduce){#working:target .mask-depth-pivot,#working:target .mask-depth-bracket {animation:none;}}\n'
    style.text = css


if __name__ == '__main__':
    ET.register_namespace('', 'http://www.w3.org/2000/svg')
    here = Path(__file__).resolve().parent
    for path in [here/'wordmark-template.svg', here.parent.parent/'public/theme/ironsmith-logo.svg']:
        root = ET.parse(path).getroot()
        configure(root)
        path.write_text(ET.tostring(root, encoding='unicode')+'\n')
