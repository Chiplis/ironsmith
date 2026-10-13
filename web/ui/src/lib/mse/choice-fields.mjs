// IDs follow MSE's depth-first choice order. A group with a default name has
// its own selectable value before its children; unnamed groups omit a prefix.
export function choiceDefinitions(record) {
  if (record.text('type') === 'boolean') return ['yes', 'no'].map(name => ({ name, primary: name }));
  const result = [];
  const visit = (entry, prefix = '', primary = null) => {
    const data = entry.record(), name = entry.inline || data.text('name');
    const children = data.all('choice');
    const full = prefix && name ? `${prefix} ${name}` : prefix || name;
    if (!children.length || data.text('default name')) result.push({ name: full, primary: primary ?? name });
    for (const child of children) visit(child, full, primary ?? name);
  };
  for (const entry of record.all('choice')) visit(entry);
  if (!record.all('choice').length) result.push({ name: record.text('name'), primary: record.text('name') });
  return result;
}

export function normalizeMultipleChoice(value, choices, empty = '') {
  const selected = new Set();
  for (const part of value.split(',')) {
    const token = part.replace(/^ +/, '');
    if (!token) continue;
    // Native MultipleChoiceValue::normalForm matches a prefix, including when
    // one choice name is a prefix of another. Keep definition order here.
    const index = choices.findIndex(choice => token.startsWith(choice.name));
    if (index >= 0) selected.add(index);
  }
  return choices.filter((_, index) => selected.has(index)).map(choice => choice.name).join(', ') || empty;
}
