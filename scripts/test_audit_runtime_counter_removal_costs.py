import unittest
from audit_runtime_counter_removal_costs import cost_roots, walk


def removal(**kwargs):
    return {"kind":"RemoveAnyCountersAmongEffect","payload":{"count":2,"counter_type":None,"filter":{"source":True},**kwargs}}


def findings(definition):
    return [row for root,path in cost_roots(definition) for row in walk(root,path)]


class CounterCostScopeTests(unittest.TestCase):
    def test_resolution_effect_is_not_a_cost(self):
        self.assertEqual(findings({"spell_effect":{"segments":[{"default_effects":[removal()]}]}}),[])

    def test_nested_oneof_retains_branches_without_duplicate_roots(self):
        definition={"abilities":[{"mana_cost":{"kind":{"OneOf":[{"kind":{"All":[{"Effect":removal()}]}},{"kind":{"All":[{"Effect":removal(count=3)}]}}]}}}]}
        rows=findings(definition)
        self.assertEqual([r['count'] for r in rows],[2,3])
        self.assertEqual(len({r['path'] for r in rows}),2)

    def test_flattened_cache_is_excluded(self):
        cost={"kind":{"All":[{"Effect":removal()}]}}
        definition={"segments":[{"default_effects":[{"kind":"NestedDefinition","payload":{"additional_cost":cost}}]}],"flattened_default_effects":[{"additional_cost":cost}]}
        self.assertEqual(len(findings(definition)),1)

    def test_old_fixed_remove_shape_is_explicitly_out_of_scope(self):
        definition={"additional_cost":{"kind":{"All":[{"RemoveCounters":{"count":1,"counter_type":"Charge"}}]}}}
        self.assertEqual(findings(definition),[])

    def test_dynamic_minimum_and_filter_fields_are_retained(self):
        definition={"additional_cost":{"kind":{"All":[{"Effect":removal(count=100,min_count=1,dynamic_count=True,filter={"other":True,"controller":"You","card_types":["Artifact"]})}]}}}
        row=findings(definition)[0]
        self.assertEqual((row['count'],row['min_count'],row['dynamic_count'],row['other'],row['controller'],row['card_types']),(100,1,True,True,'You',['Artifact']))


if __name__=='__main__':
    unittest.main()
