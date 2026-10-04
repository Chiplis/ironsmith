import unittest
from audit_runtime_self_sacrifice_costs import findings

EFFECT={'Effect':{'kind':'RemoveAnyCountersAmongEffect','payload':{}}}
def all_cost(*components):
    return {'kind':{'All':list(components)}}


class SelfSacrificeCostTests(unittest.TestCase):
    def test_same_branch_is_candidate_with_original_order(self):
        rows=list(findings({'cost':all_cost('Tap',EFFECT,'SacrificeSelf')}))
        self.assertEqual(len(rows),1)
        self.assertEqual(rows[0]['ordered_cost_shapes'],['Tap',['Effect'],'SacrificeSelf'])

    def test_alternative_branches_are_not_combined(self):
        self.assertEqual(list(findings({'cost':{'kind':{'OneOf':[all_cost('SacrificeSelf'),all_cost(EFFECT)]}}})),[])

    def test_real_candidate_inside_alternative_is_retained(self):
        rows=list(findings({'cost':{'kind':{'OneOf':[all_cost('SacrificeSelf',EFFECT),all_cost('Tap')]}}}))
        self.assertEqual(len(rows),1)
        self.assertIn('/OneOf/0/',rows[0]['path'])

    def test_resolution_sacrifice_is_outside_cost_scope(self):
        self.assertEqual(list(findings({'spell_effect':['SacrificeSelf',EFFECT]})),[])


if __name__=='__main__':
    unittest.main()
