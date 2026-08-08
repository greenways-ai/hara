from pathlib import Path

path = Path("core/rust/src/vm/machine/observation.rs")
source = path.read_text()

source = source.replace(
    "        let mut saw_multiply = false;\n",
    "        // The production compiler folds the nested multiplication.\n"
    "        // Observation must report only instructions the VM actually executes.\n",
    1,
)

multiply_block = '''                if instruction.opcode == "primitive"
                    && instruction
                        .operands
                        .contains(&InstructionOperand::Text("*".into()))
                {
                    saw_multiply = true;
                }
'''
assert multiply_block in source, "multiply observation block changed"
source = source.replace(multiply_block, "", 1)

assert "        assert!(saw_multiply);\n" in source, "multiply assertion changed"
source = source.replace("        assert!(saw_multiply);\n", "", 1)

static_old = '''        let mut machine = machine("(do (defn f [x] (+ x 1)) (f 41))");
        let (kinds, value) = run_observed(&mut machine);
'''
static_new = '''        let mut machine = machine("(do (defn f [x] (+ x 1)) (f 41))");
        let registry = crate::embedding_namespace_registry();
        let (kinds, value) = crate::core::with_namespace_registry(&registry, || {
            run_observed(&mut machine)
        });
'''
if static_old in source:
    source = source.replace(static_old, static_new, 1)
else:
    assert static_new in source, "static-call registry fixture changed"

path.write_text(source)
