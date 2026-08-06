from pathlib import Path
import re

EMITTERS = [
    Path("lib/src/tahto/model/v1/spec_hara/emit.hal"),
    Path("rust/hal-src/tahto/model/v1/spec_hara/emit.hal"),
]
TYPE_HARA = [
    Path("lib/src/tahto/runtime/basic/type_hara.hal"),
    Path("rust/hal-src/tahto/runtime/basic/type_hara.hal"),
]


def replace_once(text: str, pattern: str, replacement: str, label: str) -> str:
    updated, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"{label}: expected one replacement, got {count}")
    return updated


for path in EMITTERS:
    text = path.read_text()

    text = replace_once(
        text,
        r"\(defn lower-vector\n  \[value context\]\n  \(mapv #\(lower % context\) value\)\)",
        """(defn lower-vector
  [value context]
  (form
   (concat ['tahto.runtime.standard/arr-new]
           (map #(lower % context) value))))""",
        f"{path}: lower-vector",
    )

    text = replace_once(
        text,
        r"\(defn lower-map\n  \[value context\]\n  \(reduce-kv\n   \(fn \[out key item\]\n     \(assoc out \(lower key context\)\n                \(lower item context\)\)\)\n   \{\}\n   value\)\)",
        """(defn lower-map
  [value context]
  (form
   (reduce-kv
    (fn [out key item]
      (conj out
            (lower key context)
            (lower item context)))
    ['tahto.runtime.standard/obj-new]
    value)))""",
        f"{path}: lower-map",
    )

    text = replace_once(
        text,
        r"\(defn global-reference-call\n.*?\n\(defn lower-assignment",
        """(defn mutation-call
  [target helper arguments context]
  (form
   (concat [helper (lower target context)]
           (map #(lower % context) arguments))))

(defn lower-assignment""",
        f"{path}: mutation-call",
    )

    text = text.replace(
        "'tahto.runtime.hara/set-path!",
        "'tahto.runtime.standard/set-path!",
    )

    text = replace_once(
        text,
        r"\(defn lower-for-object\n  \[value context\]\n  \(let \[binding \(second value\)\n        pair \(first binding\)\n        collection \(lower \(second binding\) context\)\n        body \(drop 2 value\)\]\n    \(list 'reduce\n          \(list 'fn \['__tahto_ignore pair\]\n                \(lower-body body context\)\n                nil\)\n          nil\n          collection\)\)\)",
        """(defn lower-for-object
  [value context]
  (let [binding (second value)
        pair (first binding)
        collection (lower (second binding) context)
        body (drop 2 value)]
    (list 'reduce
          (list 'fn ['__tahto_ignore pair]
                (lower-body body context)
                nil)
          nil
          (list 'tahto.runtime.standard/obj-pairs collection))))""",
        f"{path}: lower-for-object",
    )

    native_operations = '''        "x:get-key" (form
                     (concat ['tahto.runtime.standard/obj-get]
                             (map lowered arguments)))
        "x:set-key" (list 'tahto.runtime.standard/obj-set!
                          (lowered (argument 0))
                          (lowered (argument 1))
                          (lowered (argument 2)))
        "x:del-key" (list 'tahto.runtime.standard/obj-delete!
                          (lowered (argument 0))
                          (lowered (argument 1)))
        "x:get-idx" (form
                     (concat ['tahto.runtime.standard/arr-get]
                             (map lowered arguments)))
        "x:set-idx" (list 'tahto.runtime.standard/arr-set!
                          (lowered (argument 0))
                          (lowered (argument 1))
                          (lowered (argument 2)))
        "x:has-key?" (let [found
                          (list 'tahto.runtime.standard/obj-has?
                                (lowered (argument 0))
                                (lowered (argument 1)))]
                      (if (> (count arguments) 2)
                        (list '= (lowered (argument 2)) found)
                        found))
        "x:copy-key" (list 'tahto.runtime.standard/obj-set!
                           (lowered (argument 0))
                           (lowered (argument 2))
                           (list 'tahto.runtime.standard/obj-get
                                 (lowered (argument 1))
                                 (lowered (argument 2))))
        "x:obj-keys" (list 'tahto.runtime.standard/obj-keys
                           (lowered (argument 0)))
        "x:obj-vals" (list 'tahto.runtime.standard/obj-vals
                           (lowered (argument 0)))
        "x:obj-pairs" (list 'tahto.runtime.standard/obj-pairs
                            (lowered (argument 0)))
        "x:obj-clone" (list 'tahto.runtime.standard/obj-clone
                            (lowered (argument 0)))
        "x:obj-assign" (list 'tahto.runtime.standard/obj-assign!
                             (lowered (argument 0))
                             (lowered (argument 1)))
        "x:arr-clone" (list 'tahto.runtime.standard/arr-clone
                            (lowered (argument 0)))
        "x:arr-push" (list 'tahto.runtime.standard/arr-push!
                           (lowered (argument 0))
                           (lowered (argument 1)))
        "x:arr-pop" (list 'tahto.runtime.standard/arr-pop!
                          (lowered (argument 0)))
        "x:arr-push-first" (list 'tahto.runtime.standard/arr-push-first!
                                 (lowered (argument 0))
                                 (lowered (argument 1)))
        "x:arr-pop-first" (list 'tahto.runtime.standard/arr-pop-first!
                                (lowered (argument 0)))
        "x:arr-remove" (list 'tahto.runtime.standard/arr-remove!
                             (lowered (argument 0))
                             (lowered (argument 1)))
        "x:arr-insert" (list 'tahto.runtime.standard/arr-insert!
                             (lowered (argument 0))
                             (lowered (argument 1))
                             (lowered (argument 2)))
        "x:arr-slice" (form
                       (concat ['tahto.runtime.standard/arr-slice]
                               (map lowered arguments)))
        "x:arr-reverse" (list 'tahto.runtime.standard/arr-reverse!
                              (lowered (argument 0)))
        "x:arr-map" (list 'tahto.runtime.standard/arr-map
                          (lowered (argument 0))
                          (lowered (argument 1)))
        "x:arr-filter" (list 'tahto.runtime.standard/arr-filter
                             (lowered (argument 0))
                             (lowered (argument 1)))
        "x:arr-foldl" (list 'tahto.runtime.standard/arr-fold-left
                            (lowered (argument 0))
                            (lowered (argument 1))
                            (lowered (argument 2)))
        "x:arr-foldr" (list 'tahto.runtime.standard/arr-fold-right
                            (lowered (argument 0))
                            (lowered (argument 1))
                            (lowered (argument 2)))
        "x:arr-some" (list 'any?
                           (lowered (argument 1))
                           (lowered (argument 0)))
        "x:arr-every" (list 'every?
                            (lowered (argument 1))
                            (lowered (argument 0)))
        "x:to-string"'''

    text = replace_once(
        text,
        r'        "x:get-key".*?        "x:to-string"',
        native_operations,
        f"{path}: native operations",
    )

    text = text.replace(
        '''        "x:is-object?" (list 'map? (lowered (argument 0)))''',
        '''        "x:is-object?" (list 'tahto.runtime.standard/obj?
                             (lowered (argument 0)))''',
    )
    text = text.replace(
        '''        "x:is-array?" (list 'vector? (lowered (argument 0)))''',
        '''        "x:is-array?" (list 'tahto.runtime.standard/arr?
                            (lowered (argument 0)))''',
    )

    if "tahto.runtime.hara" in text:
        raise SystemExit(f"{path}: old runtime namespace remains")
    path.write_text(text)

for path in TYPE_HARA:
    text = path.read_text()
    old = "            [tahto.runtime.hara]))"
    new = "            [tahto.runtime.standard]))"
    if text.count(old) != 1:
        raise SystemExit(f"{path}: expected one runtime require, got {text.count(old)}")
    path.write_text(text.replace(old, new))
