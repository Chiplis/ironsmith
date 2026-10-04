import unittest
from audit_runtime_extended_cost_dependencies import findings

def choose(tag='cost'):
 return {'Effect':{'kind':'ChooseObjectsEffect','payload':{'tag':tag,'count':{'min':1,'max':1}}}}
def consume(tag='cost'):
 return {'Effect':{'kind':'ExileEffect','payload':{'target':{'Tagged':tag}}}}
def wrap(parts,activated=False):
 cost={'kind':{'All':parts}}
 return {'abilities':[{'kind':{'Activated':{'mana_cost':cost}}}]} if activated else {'additional_cost':cost}
class CostDependencyTests(unittest.TestCase):
 def test_spell_pair(self):
  rows=list(findings(wrap([choose(),consume()])))
  self.assertEqual(len(rows),1);self.assertEqual(rows[0]['distance'],1)
 def test_adjacent_activation_excluded(self):
  self.assertEqual(list(findings(wrap([choose(),consume()],True))),[])
 def test_nonadjacent_activation(self):
  rows=list(findings(wrap([choose(),'PayLife',consume()],True)))
  self.assertEqual(len(rows),1);self.assertEqual(rows[0]['distance'],2)
 def test_unrelated_tag_excluded(self):
  self.assertEqual(list(findings(wrap([choose(),consume('other')]))),[])
 def test_same_tag_reselection_stops_earlier_binding(self):
  rows=list(findings(wrap([choose(),choose(),consume()])))
  self.assertEqual(len(rows),1);self.assertTrue(rows[0]['path'].endswith('/1'))
 def test_oneof_branches_do_not_share_tags(self):
  definition={'additional_cost':{'kind':{'OneOf':[{'kind':{'All':[choose()]}},{'kind':{'All':[consume()]}}]}}}
  self.assertEqual(list(findings(definition)),[])
 def test_flattened_cache_is_ignored(self):
  definition={'flattened_default_effects':[wrap([choose(),consume()])]}
  self.assertEqual(list(findings(definition)),[])
if __name__=='__main__':unittest.main()
