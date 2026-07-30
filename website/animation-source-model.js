const STRING = '"((?:\\\\.|[^"\\\\])*)"';

export function projectAnimationSource(source) {
  if (!source.includes("(def animation-pipeline")) {
    throw new Error("Source must define animation-pipeline");
  }
  const selected = match(source, new RegExp(`"selected"\\s+${STRING}`), "selected character");
  const actionsMatch = source.match(/"actions"\s+\[([^\]]*)\]/s);
  if (!actionsMatch) throw new Error("animation-pipeline needs an actions vector");
  const actions = [...actionsMatch[1].matchAll(new RegExp(STRING, "g"))]
    .map((entry) => decode(entry[1]));
  if (!actions.length) throw new Error("Choose at least one action");
  return { selected, actions };
}

export function patchAnimationCharacter(source, character) {
  return replace(source, /("selected"\s+)"(?:\\.|[^"\\])*"/, `$1${JSON.stringify(character)}`);
}

export function patchAnimationActions(source, actions) {
  if (!actions.length) throw new Error("Choose at least one action");
  return replace(source, /("actions"\s+)\[[^\]]*\]/s,
    `$1[${actions.map((action) => JSON.stringify(action)).join(" ")}]`);
}

function match(source, pattern, label) {
  const found = source.match(pattern);
  if (!found) throw new Error(`animation-pipeline needs ${label}`);
  return decode(found[1]);
}

function decode(value) {
  return JSON.parse(`"${value}"`);
}

function replace(source, pattern, replacement) {
  if (!pattern.test(source)) throw new Error("Animation source shape is invalid");
  return source.replace(pattern, replacement);
}
