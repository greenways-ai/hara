from pathlib import Path
import re

PATHS = [
    Path("lib/src/tahto/model/v1/spec_hara/emit.hal"),
    Path("rust/hal-src/tahto/model/v1/spec_hara/emit.hal"),
]


def replace_once(text: str, pattern: str, replacement: str, label: str) -> str:
    updated, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"{label}: expected one replacement, got {count}")
    return updated


for path in PATHS:
    text = path.read_text()
    text = replace_once(
        text,
        r"\(defn lower-vector\n  \[value context\]\n  \(mapv #\(lower % context\) value\)\)",
        """(defn lower-vector
  [value context]
  (form
   (concat ['tahto.runtime.hara/arr-new]
           (map #(lower % context) value))))""",
        f"{path}: lower-vector",
    )
    text = replace_once(
        text,
        r"\(defn lower-map\n  \[value context\]\n  \(reduce-kv\n.*?\n   value\)\)",
        """(defn lower-map
  [value context]
  (form
   (reduce-kv
    (fn [out key item]
      (conj out
            (lower key context)
            (lower item context)))
    ['tahto.runtime.hara/obj-new]
    value)))""",
        f"{path}: lower-map",
    )
    text = replace_once(
        text,
        r'        "x:get-key".*?(?=\n        "x:set-key")',
        """        "x:get-key" (if (> (count arguments) 2)
                      (list 'tahto.runtime.hara/obj-get
                            (lowered (argument 0))
                            (lowered (argument 1))
                            (lowered (argument 2)))
                      (list 'tahto.runtime.hara/obj-get
                            (lowered (argument 0))
                            (lowered (argument 1))))""",
        f"{path}: x:get-key",
    )
    text = replace_once(
        text,
        r'        "x:get-idx".*?(?=\n        "x:set-idx")',
        """        "x:get-idx" (list 'tahto.runtime.hara/arr-get
                          (lowered (argument 0))
                          (lowered (argument 1)))""",
        f"{path}: x:get-idx",
    )
    text = replace_once(
        text,
        r'        "x:has-key\?".*?(?=\n        "x:obj-keys")',
        """        "x:has-key?" (let [found
                          (list 'tahto.runtime.hara/obj-has?
                                (lowered (argument 0))
                                (lowered (argument 1)))]
                      (if (> (count arguments) 2)
                        (list '= (lowered (argument 2)) found)
                        found))""",
        f"{path}: x:has-key?",
    )
    text = replace_once(
        text,
        r'        "x:obj-keys".*?(?=\n        "x:obj-assign")',
        """        "x:obj-keys" (list 'tahto.runtime.hara/obj-keys
                           (lowered (argument 0)))
        "x:obj-vals" (list 'tahto.runtime.hara/obj-vals
                           (lowered (argument 0)))
        "x:obj-clone" (list 'tahto.runtime.hara/obj-clone
                            (lowered (argument 0)))""",
        f"{path}: object reads",
    )
    text = replace_once(
        text,
        r'        "x:arr-clone".*?(?=\n        "x:arr-push")',
        """        "x:arr-clone" (list 'tahto.runtime.hara/arr-clone
                            (lowered (argument 0)))""",
        f"{path}: x:arr-clone",
    )
    text = replace_once(
        text,
        r'        "x:arr-map".*?(?=\n        "x:arr-some")',
        """        "x:arr-map" (list 'tahto.runtime.hara/arr-map
                          (lowered (argument 0))
                          (lowered (argument 1)))
        "x:arr-filter" (list 'tahto.runtime.hara/arr-filter
                             (lowered (argument 0))
                             (lowered (argument 1)))
        "x:arr-foldl" (list 'tahto.runtime.hara/arr-fold-left
                            (lowered (argument 0))
                            (lowered (argument 1))
                            (if (> (count arguments) 2)
                              (lowered (argument 2))
                              nil))""",
        f"{path}: array transforms",
    )
    text = replace_once(
        text,
        r'        "x:is-object\?".*?\n        "x:is-array\?".*?\)',
        """        "x:is-object?" (list 'tahto.runtime.hara/obj?
                             (lowered (argument 0)))
        "x:is-array?" (list 'tahto.runtime.hara/arr?
                            (lowered (argument 0)))""",
        f"{path}: native predicates",
    )
    path.write_text(text)
