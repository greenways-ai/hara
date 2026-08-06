package hara.truffle;

import hara.lang.block.Parser;
import hara.lang.data.Keyword;
import hara.lang.data.List;
import hara.lang.protocol.IMetadata;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Comparator;
import java.util.HashSet;
import java.util.Set;

/** Imports annotated static methods without exposing unannotated implementation helpers. */
final class HaraStaticLibrary {
  private HaraStaticLibrary() {}

  static void install(HaraContext context, String namespace, Class<?> implementation) {
    Method[] methods = implementation.getDeclaredMethods();
    Arrays.sort(methods, Comparator.comparing(Method::getName));
    Set<String> exported = new HashSet<>();
    for (Method method : methods) {
      HaraExport export = method.getAnnotation(HaraExport.class);
      if (export == null) continue;
      validate(method, export);
      if (!exported.add(export.name())) {
        throw new HaraException(
            "Duplicate Hara export " + namespace + "/" + export.name() + " in "
                + implementation.getName());
      }
      IMetadata metadata = metadata(export);
      switch (export.kind()) {
        case FUNCTION:
          context.defineLibraryFunction(
              namespace,
              export.name(),
              values -> HaraPersistentValues.normalize(invoke(method, context, values)),
              metadata);
          break;
        case VALUE:
          context.defineLibraryValue(
              namespace,
              export.name(),
              HaraPersistentValues.normalize(invoke(method, context)),
              metadata);
          break;
        case MACRO:
          context.defineLibraryMacro(
              namespace,
              export.name(),
              invocation -> invoke(method, context, invocation),
              metadata,
              export.intrinsic());
          break;
        default:
          throw new HaraException("Unsupported Hara export kind: " + export.kind());
      }
    }
  }

  private static void validate(Method method, HaraExport export) {
    if (!Modifier.isPublic(method.getModifiers()) || !Modifier.isStatic(method.getModifiers())) {
      throw new HaraException("@HaraExport requires a public static method: " + method);
    }
    Class<?>[] parameters = method.getParameterTypes();
    boolean valid;
    switch (export.kind()) {
      case FUNCTION:
        valid = Arrays.equals(parameters, new Class<?>[] {HaraContext.class, Object[].class});
        break;
      case VALUE:
        valid = Arrays.equals(parameters, new Class<?>[] {HaraContext.class});
        break;
      case MACRO:
        valid = Arrays.equals(parameters, new Class<?>[] {HaraContext.class, List.class});
        break;
      default:
        valid = false;
    }
    if (!valid) {
      throw new HaraException("Invalid signature for " + export.kind() + " export: " + method);
    }
  }

  private static Object invoke(Method method, Object... arguments) {
    try {
      return method.invoke(null, arguments);
    } catch (InvocationTargetException error) {
      Throwable cause = error.getCause();
      if (cause instanceof RuntimeException) throw (RuntimeException) cause;
      throw new HaraException(
          "Static Hara export failed: " + method.getName() + " (" + cause.getMessage() + ")");
    } catch (ReflectiveOperationException error) {
      throw new HaraException(
          "Unable to invoke static Hara export " + method.getName() + ": " + error.getMessage());
    }
  }

  private static IMetadata metadata(HaraExport export) {
    ArrayList<Object> entries = new ArrayList<>();
    if (!export.doc().isEmpty()) {
      entries.add(Keyword.create("doc"));
      entries.add(export.doc());
    }
    if (export.arglists().length > 0) {
      ArrayList<Object> arglists = new ArrayList<>();
      for (String arglist : export.arglists()) arglists.add(Parser.parseString(arglist));
      entries.add(Keyword.create("arglists"));
      entries.add(hara.lang.data.List.Standard.from(null, arglists.toArray()));
    }
    return entries.isEmpty()
        ? hara.lang.data.Map.Standard.EMPTY
        : hara.lang.data.Map.Standard.from(null, entries.toArray());
  }
}
