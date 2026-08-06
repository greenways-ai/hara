package hara.kernel.base;

import hara.kernel.jvm.JvmFlavorProvider;
import hara.kernel.jvm.JvmNativeFunction;
import hara.lang.data.Symbol;

/** Installs the explicit capability-gated JVM library namespaces. */
final class JvmNativeLibraries {
  private JvmNativeLibraries() {}

  static void install(RT.Instance<?> runtime) {
    Namespace reflect = runtime.getNamespace(Symbol.create("hara.native.jvm.reflect"));
    reflect.mappings.put(
        Symbol.create("type"),
        fn("reflect/type", v -> provider(runtime).type(v[0], runtime._nativeAccess), 1));
    reflect.mappings.put(
        Symbol.create("name"),
        fn("reflect/name", v -> provider(runtime).typeName(v[0], runtime._nativeAccess), 1));
    reflect.mappings.put(
        Symbol.create("instance?"),
        fn(
            "reflect/instance?",
            v -> provider(runtime).isInstance(v[0], v[1], runtime._nativeAccess),
            2));
    reflect.mappings.put(
        Symbol.create("fields"),
        fn("reflect/fields", v -> provider(runtime).fields(v[0], runtime._nativeAccess), 1));
    reflect.mappings.put(
        Symbol.create("methods"),
        fn("reflect/methods", v -> provider(runtime).methods(v[0], runtime._nativeAccess), 1));
    Namespace classpath = runtime.getNamespace(Symbol.create("hara.native.jvm.classpath"));
    classpath.mappings.put(
        Symbol.create("paths"),
        fn("classpath/paths", v -> provider(runtime).classPath(runtime._nativeAccess), 0));
    classpath.mappings.put(
        Symbol.create("add!"),
        fn(
            "classpath/add!",
            v -> provider(runtime).addClassPath(String.valueOf(v[0]), runtime._nativeAccess),
            1));
    Namespace compiler = runtime.getNamespace(Symbol.create("hara.native.jvm.compiler"));
    compiler.mappings.put(
        Symbol.create("compile"),
        fn("compiler/compile", v -> provider(runtime).compile(v[0], runtime._nativeAccess), 1));
    compiler.mappings.put(
        Symbol.create("define!"),
        fn(
            "compiler/define!",
            v -> provider(runtime).defineClass(bytecode(v[0]), runtime._nativeAccess),
            1));
    compiler.mappings.put(
        Symbol.create("compile!"),
        fn(
            "compiler/compile!",
            v -> {
              JvmFlavorProvider provider = provider(runtime);
              return provider.defineClass(
                  provider.compile(v[0], runtime._nativeAccess), runtime._nativeAccess);
            },
            1));
  }

  private static JvmFlavorProvider provider(RT.Instance<?> runtime) {
    return JvmNativeFunction.provider(runtime);
  }

  private static byte[] bytecode(Object value) {
    if (value instanceof byte[]) return (byte[]) value;
    throw new hara.lang.base.Ex.Runtime("compiler/define! expects bytes");
  }

  private static Var fn(
      String name, java.util.function.Function<Object[], Object> body, int arity) {
    return new Var(
        name,
        new JvmNativeFunction(
            name,
            values -> {
              JvmNativeFunction.requireArity(name, values, arity);
              return body.apply(values);
            }));
  }
}
