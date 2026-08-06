package hara.kernel;

import hara.kernel.base.Env;
import hara.kernel.base.Macro;
import hara.kernel.base.Module;
import hara.lang.base.Iter;
import hara.lang.data.Symbol;
import hara.kernel.base.Var;
import org.junit.Test;

import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.Map;

public class ListSymbols {

  @Test
  public void listAllSymbols() {
    // Classes to scan (from Env.java)
    Class[] classes =
        new Class[] {
          hara.kernel.builtin.BuiltinBasic.class,
          hara.kernel.builtin.BuiltinCheck.class,
          hara.kernel.builtin.BuiltinRef.class,
          hara.kernel.builtin.BuiltinCollection.class,
          hara.kernel.builtin.BuiltinInterop.class,
          hara.kernel.builtin.BuiltinLambda.class,
          hara.kernel.builtin.BuiltinOps.class,
          hara.kernel.builtin.BuiltinRuntime.class,
          hara.kernel.builtin.BuiltinStruct.class,
          hara.kernel.builtin.BuiltinTime.class,
          hara.kernel.builtin.BuiltinNamespace.class,
          hara.kernel.builtin.BuiltinUtil.class
        };

    System.out.println("# Complete Symbol List");

    for (Class cls : classes) {
      printSymbolsForClass(cls);
    }

    // Macros
    for (Class cls : Macro.class.getDeclaredClasses()) {
      printSymbolsForClass(cls);
    }
  }

  private void printSymbolsForClass(Class cls) {
    Module.Ns nsAnn = (Module.Ns) cls.getAnnotation(Module.Ns.class);
    String nsName = (nsAnn != null) ? nsAnn.name() : "global";
    String tagName = (nsAnn != null) ? nsAnn.tag() : cls.getSimpleName();

    System.out.println("\n## " + cls.getSimpleName() + " (" + nsName + ")");

    List<Method> methods = new ArrayList<>();
    for (Method m : cls.getDeclaredMethods()) {
      if (m.isAnnotationPresent(Module.Fn.class)) {
        methods.add(m);
      }
    }

    methods.sort(Comparator.comparing(m -> m.getAnnotation(Module.Fn.class).name()));

    List<String> seen = new ArrayList<>();

    for (Method m : methods) {
      Module.Fn fn = m.getAnnotation(Module.Fn.class);
      String name = fn.name();
      if (!seen.contains(name)) {
        seen.add(name);
        System.out.println("\n### `" + name + "`");
        if (!fn.doc().isEmpty()) {
          System.out.println(fn.doc());
        }
        // We could print arglists too if we parsed them, but let's stick to names for
        // now.
        System.out.println("```clojure");
        System.out.println("(" + name + " ...)");
        System.out.println("```");
      }
    }
  }
}
