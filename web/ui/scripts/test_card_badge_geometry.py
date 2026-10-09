import unittest
from PIL import Image,ImageDraw
from card_badge_geometry import badge_digit_box
class BadgeGeometryTests(unittest.TestCase):
 def test_digit_component_is_separate_from_star_border_and_neighboring_decoration(self):
  image=Image.new('L',(100,100),0);draw=ImageDraw.Draw(image)
  draw.rectangle((0,0,99,99),outline=255,width=3)
  draw.rectangle((10,10,25,25),fill=255)
  draw.rectangle((63,55,68,64),fill=255)
  self.assertEqual(badge_digit_box(image),(55,47,77,73))
 def test_no_isolated_digit_does_not_invent_a_badge_box(self):
  image=Image.new('L',(100,100),0);ImageDraw.Draw(image).rectangle((0,0,99,99),outline=255,width=3)
  self.assertIsNone(badge_digit_box(image))
if __name__=='__main__':unittest.main()
