import unittest
from audit_runtime_negative_tag_choices import screen


class NegativeTagRoutes(unittest.TestCase):
    def spec(self, relation='IsNotTaggedObject'):
        return {'Object': {'tagged_constraints': [{'relation': relation, 'tag': '__it__'}]}}

    def test_generic_damage_and_scratch_tag_are_not_excluded(self):
        rows=list(screen({'kind':'DealDamageEffect','payload':{'target':self.spec()}}))
        self.assertEqual(rows[0]['route'],'generic_negative_tag_candidate_pool_risk')

    def test_targeted_form_is_separate(self):
        rows=list(screen({'kind':'DealDamageEffect','payload':{'target':{'Target':self.spec()}}}))
        self.assertEqual(rows[0]['route'],'targeted_form_excluded')

    def test_continuous_surface_wrapper_uses_different_resolver(self):
        rows=list(screen({'kind':'ApplyContinuousEffect','payload':{'target_spec':{'SurfaceHinted':{'spec':self.spec()}}}}))
        self.assertEqual(rows[0]['route'],'continuous_effect_whole_battlefield_filter_route')

    def test_skip_flattened_and_positive_filter(self):
        effect={'kind':'MoveToZoneEffect','payload':{'target':self.spec()}}
        self.assertEqual(len(list(screen({'flattened_default_effects':[effect],'segments':[effect]}))),1)
        self.assertEqual(list(screen({'kind':'MoveToZoneEffect','payload':{'target':self.spec('IsTaggedObject')}})),[])

    def test_counted_form_not_generic_direct_reference(self):
        rows=list(screen({'kind':'DealDamageEffect','payload':{'target':{'WithCount':[self.spec(),{'min':1,'max':1}]}}}))
        self.assertEqual(rows[0]['route'],'counted_choice_route_requires_separate_review')


if __name__=='__main__': unittest.main()
