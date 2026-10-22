import tempfile
import unittest
from pathlib import Path
import xml.etree.ElementTree as ET
from fontTools.ttLib import TTFont
from generate import generate, NS

FONTS = Path(__file__).resolve().parents[2] / 'src/assets/fonts'

class FontWordmarkTests(unittest.TestCase):
    def test_distinct_fonts_change_outline_and_strike_without_stretching(self):
        with tempfile.TemporaryDirectory() as tmp:
            results = []
            paths = []
            for name in ['Plantin-Semibold.otf', 'Matrix-Bold.ttf']:
                out = Path(tmp)/f'{name}.svg'
                metadata = generate(FONTS/name, out)
                root = ET.parse(out).getroot()
                transforms = [p.get('transform') for p in root.find('.//*[@class="font-wordmark"]')]
                for transform in transforms:
                    values = transform[7:-1].split()
                    self.assertAlmostEqual(float(values[0]), -float(values[3]))
                # Scaling around the baseline keeps the smith standing with the letters.
                self.assertAlmostEqual(1004*metadata['smithScale']+metadata['smithOffset'][1],1004)
                self.assertAlmostEqual(763.1*metadata['smithScale']+metadata['smithOffset'][1], metadata['strike'][1])
                self.assertEqual(len(root.findall('.//*[@class="font-wordmark"]/*')),7+1)
                self.assertIn('prefers-reduced-motion', root.find(NS+'style').text)
                results.append(metadata['strike'])
                paths.append(root.find(NS+'style').text.split('@keyframes forge-letter')[1])
            self.assertNotEqual(results[0], results[1])
            self.assertNotEqual(paths[0],paths[1])

    def test_missing_h_fails_without_creating_output(self):
        with tempfile.TemporaryDirectory() as tmp:
            font = TTFont(FONTS/'Matrix-Bold.ttf')
            for table in font['cmap'].tables:
                table.cmap.pop(ord('h'),None)
            broken = Path(tmp)/'missing.ttf'
            font.save(broken)
            output = Path(tmp)/'missing.svg'
            with self.assertRaisesRegex(ValueError, 'missing characters: h'):
                generate(broken, output)
            self.assertFalse(output.exists())

    def test_static_font_rejects_variable_axes(self):
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaisesRegex(ValueError, 'variable font'):
                generate(FONTS/'Matrix-Bold.ttf', Path(tmp)/'bad.svg', {'wght':700})

if __name__ == '__main__':
    unittest.main()
