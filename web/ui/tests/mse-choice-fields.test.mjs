import test from 'node:test';
import assert from 'node:assert/strict';
import { MsePackageStore } from '../src/lib/mse/packages.mjs';
import { MseTemplateContext, loadMseTemplate } from '../src/lib/mse/template.mjs';
import { text } from '../src/lib/mse/script-values.mjs';

async function context() {
  const files = {
    'choices.mse-style/style': 'game: choices\ncard width: 375\ncard height: 523\n',
    'choices.mse-game/game': `card field:
\ttype: boolean
\tname: flag
card field:
\ttype: choice
\tname: group
\tchoice:
\t\tname: outer
\t\tchoice: first
\t\tchoice:
\t\t\tname: nested
\t\t\tdefault name: Standard
\t\t\tchoice: child
\tchoice: other
card field:
\ttype: choice
\tname: group default
\tchoice:
\t\tname: group
\t\tdefault name: Standard
\t\tchoice: child
card field:
\ttype: choice
\tname: implicit
card field:
\ttype: multiple choice
\tname: colors
\tchoice: white
\tchoice: blue
\tchoice: black
\tempty choice: colorless
card field:
\ttype: multiple choice
\tname: exclusive
\tinitial: white, blue
\tchoice: white
\tchoice: blue
\tscript: exclusive_choice(input, choices:"white, blue")
card field:
\ttype: text
\tname: primary
\tscript: primary_choice(card.group)
`,
  };
  const template = await loadMseTemplate(new MsePackageStore({ readText: async path => files[path.key] }), 'choices.mse-style');
  return new MseTemplateContext(template).initialize();
}

test('choice defaults follow native group IDs and primary_choice finds the top-level group', async () => {
  const c = await context();
  assert.equal(text(c.field('card', 'group')), 'outer first');
  assert.equal(text(c.field('card', 'group_default')), 'group');
  assert.equal(text(c.field('card', 'implicit')), 'implicit');
  assert.equal(text(c.field('card', 'flag')), 'yes');
  assert.equal(c.runtime.run('primary_choice(card.flag)'), 'yes');
  assert.equal(text(c.field('card', 'primary')), 'outer');
  c.setField('card', 'group', 'outer nested child');
  assert.equal(text(c.field('card', 'primary')), 'outer');
  c.setField('card', 'group', 'other');
  assert.equal(text(c.field('card', 'primary')), 'other');
});

test('multiple choices normalize duplicates, unknown choices, prefixes and declaration order', async () => {
  const c = await context();
  assert.equal(text(c.field('card', 'colors')), 'colorless');
  c.setField('card', 'colors', ' black, white, black, unknown, bluetail');
  assert.equal(text(c.field('card', 'colors')), 'white, blue, black');
  c.setField('card', 'colors', 'unknown');
  assert.equal(text(c.field('card', 'colors')), 'colorless');
});

test('last_change is empty initially and explicitly selects the preferred option after an edit', async () => {
  const c = await context();
  assert.equal(text(c.field('card', 'exclusive')), 'white');
  c.setField('card', 'exclusive', 'white, blue', { lastChange: 'blue' });
  assert.equal(text(c.field('card', 'exclusive')), 'blue');
  c.invalidate();
  assert.equal(text(c.field('card', 'exclusive')), 'blue');
});

test('choice-family script results and supplied values retain native string storage',async()=>{
  for(const type of ['choice','boolean','multiple choice','slider']){
    const files={
      'scalar.mse-game/game':`card field:\n\ttype: ${type}\n\tname: value\n\tchoice: 12\n\tchoice: 24\n\tdefault: 12\n\tscript: if input == "12" then 24 else input\ncard field:\n\ttype: text\n\tname: check\n\tdefault: if card.value == "24" then "string" else "wrong"\n`,
      'scalar.mse-style/style':'game: scalar\n',
    };
    const template=await loadMseTemplate(new MsePackageStore({readText:async p=>files[p.key]}),'scalar.mse-style');
    for(const fieldUpdateMode of ['lazy','ordered']){
      const context=new MseTemplateContext(template,{fieldUpdateMode}).initialize();
      assert.equal(text(context.field('card','check')),'string',type);
      const supplied=new MseTemplateContext(template,{fieldUpdateMode,card:{value:24}}).initialize();
      assert.equal(text(supplied.field('card','check')),'string',type);
      supplied.setField('card','value',12);
      assert.equal(text(supplied.field('card','check')),'string',type);
    }
  }
});

test('failed choice script conversion retains the last valid value',async()=>{
  const files={
    'retain.mse-game/game':'card field:\n\ttype: choice\n\tname: value\n\tinitial: original\n\tdefault: "defaulted"\n\tscript: to_image("unused.png")\n',
    'retain.mse-style/style':'game: retain\n',
  };
  const template=await loadMseTemplate(new MsePackageStore({readText:async p=>files[p.key]}),'retain.mse-style');
  for(const fieldUpdateMode of ['lazy','ordered']){
    const context=new MseTemplateContext(template,{fieldUpdateMode}).initialize();
    assert.equal(text(context.field('card','value')),'defaulted');
    assert.equal(context.diagnostics[0].code,'script-conversion');
  }
});

test('primary_choice accepts slider values and tracks their top-level choice after edits',async()=>{
  const files={
    'slider-primary.mse-game/game':`card field:
\ttype: slider
\tname: amount
\tminimum: 0
\tmaximum: 10
\tincrement: 2
\tinitial: 4
\tchoice:
\t\tname: custom
\t\tchoice: amount
card field:
\ttype: text
\tname: primary
\tscript: primary_choice(card.amount)
`,
    'slider-primary.mse-style/style':'game: slider-primary\n',
  };
  const template=await loadMseTemplate(new MsePackageStore({readText:async p=>files[p.key]}),'slider-primary.mse-style');
  for(const fieldUpdateMode of ['lazy','ordered']){
    const context=new MseTemplateContext(template,{fieldUpdateMode}).initialize();
    assert.equal(text(context.field('card','primary')),'4');
    context.setField('card','amount','custom amount');assert.equal(text(context.field('card','primary')),'custom');
    context.setField('card','amount','10');assert.equal(text(context.field('card','primary')),'10');
    context.setField('card','amount','unknown');assert.equal(text(context.field('card','primary')),'true');
    assert.equal(context.runtime.run('primary_choice(card.amount)'),true);
    assert.throws(()=>context.runtime.run('primary_choice(card.primary)'),{code:'choice-field-type'});
    assert.deepEqual(context.diagnostics,[]);
  }
});
