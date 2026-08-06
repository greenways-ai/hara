from pathlib import Path

PATHS = [
    Path("lib/src/tahto/model/v1/spec_hara/emit.hal"),
    Path("rust/hal-src/tahto/model/v1/spec_hara/emit.hal"),
]

OLD = '''        "x:is-array?" (list 'tahto.runtime.hara/arr?
                            (lowered (argument 0)))))
        "x:apply"'''

NEW = '''        "x:is-array?" (list 'tahto.runtime.hara/arr?
                            (lowered (argument 0)))
        "x:apply"'''

for path in PATHS:
    text = path.read_text()
    count = text.count(OLD)
    if count != 1:
        raise SystemExit(f"{path}: expected native predicate closure once, got {count}")
    path.write_text(text.replace(OLD, NEW))
