import unittest
from audit_runtime_choose_consume_costs import findings


def choose(tag='chosen', minimum=1, maximum=1, dynamic=False):
    return {'Effect':{'kind':'ChooseObjectsEffect','payload':{'tag':tag,'count':{'min':minimum,'max':maximum,'dynamic_x':dynamic}}}}

def consume(tag='chosen'):
    return {'Effect':{'kind':'ExileEffect','payload':{'spec':{'Tagged':tag}}}}

def definition(components):
    return {'abilities':[{'kind':{'Activated':{'mana_cost':{'kind':{'All':components}}}}}]}


class ChooseConsumeCosts(unittest.TestCase):
    def test_adjacent_matching_tag_is_one_candidate(self):
        rows=list(findings(definition([choose(),consume()])))
        self.assertEqual(len(rows),1);self.assertEqual(rows[0]['count_shape'],'single')
    def test_different_tags_are_not_paired(self):
        self.assertEqual(list(findings(definition([choose(),consume('other')]))),[])
    def test_alternative_boundaries_are_not_paired(self):
        d=definition([]);d['abilities'][0]['kind']['Activated']['mana_cost']['kind']={'OneOf':[{'kind':{'All':[choose()]}},{'kind':{'All':[consume()]}}]}
        self.assertEqual(list(findings(d)),[])
    def test_resolution_and_spell_costs_excluded(self):
        self.assertEqual(list(findings({'spell_effect':[choose(),consume()],'additional_cost':{'kind':{'All':[choose(),consume()]}}})),[])
    def test_dynamic_and_fixed_counts_remain_distinct(self):
        self.assertEqual(list(findings(definition([choose(minimum=0,maximum=None,dynamic=True),consume()])))[0]['count_shape'],'dynamic_x')
        self.assertEqual(list(findings(definition([choose(minimum=3,maximum=3),consume()])))[0]['count_shape'],'fixed_multiple')
    def test_filter_tagged_consumer_recognized(self):
        c={'Effect':{'kind':'SacrificePlayerEffect','payload':{'filter':{'tagged_constraints':[{'tag':'chosen','relation':'IsTaggedObject'}]}}}}
        self.assertEqual(len(list(findings(definition([choose(),c])))),1)
    def test_nonadjacent_pair_excluded(self):
        self.assertEqual(list(findings(definition([choose(),'Tap',consume()]))),[])

if __name__=='__main__':unittest.main()
