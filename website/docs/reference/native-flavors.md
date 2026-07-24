# Native flavor provider contract

Hara L0 is evaluator- and platform-independent. A runtime may install native flavor providers,
but portable code does not acquire native semantics merely by using a core form.

A namespace selects a provider explicitly with `(:flavor :name)`. Selection is local to that
namespace, is not inherited by required namespaces, and grants no authority. An embedding runtime
separately grants reflection/invocation, classpath mutation, and compilation/class-definition
capabilities.

Providers receive evaluated values at generic evaluator hooks: namespace clauses, native type and
qualified-symbol resolution, construction, member reads/writes/calls, indexed steps, and exception
matching. Providers must normalize failures into stable native-flavor errors while preserving the
original cause. Unsupported providers and denied capabilities must fail deterministically.

Executable Hara source uses `.hal`. `.hara` is retained only for historical descriptors; current
projects use executable `project.hal` descriptors. `.hrl` and `.hara` are not executable module
extensions.
