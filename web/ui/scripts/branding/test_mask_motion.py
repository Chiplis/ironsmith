import math
import unittest
from mask_motion import OPEN_ANGLE, angle_at, matrix, project, rotate_point


class MaskMotionTests(unittest.TestCase):
    def test_rigid_shell_distances_preserved_in_three_dimensions(self):
        points = [(612, 343), (883, 358), (857, 699), (631, 679), (932, 500)]
        initial = [rotate_point(*p, 0) for p in points]
        for degree in range(81):
            posed = [rotate_point(*p, degree) for p in points]
            for i in range(len(points)):
                for j in range(i):
                    self.assertAlmostEqual(math.dist(initial[i], initial[j]), math.dist(posed[i], posed[j]), places=8)

    def test_closed_pose_matches_artwork_and_never_turns_inside_out(self):
        for point in [(612, 343), (883, 358), (857, 699), (631, 679)]:
            projected = project(rotate_point(*point, 0))
            for a, b in zip(point, projected):
                self.assertAlmostEqual(a, b)
        for percent in range(101):
            a, b, c, d, _, _ = matrix(angle_at(percent))
            self.assertGreater(a*d-b*c, 0)
        self.assertEqual(angle_at(0), angle_at(100))
        self.assertEqual(angle_at(50), 0)
        # Raised lower edge clears the eyes rather than crossing the face.
        self.assertLess(project(rotate_point(857, 699, OPEN_ANGLE))[1], 480)

if __name__ == '__main__':
    unittest.main()
