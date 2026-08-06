package hara.kernel.jvm;

import hara.compiler.Compiler;
import hara.kernel.NativeMode;
import hara.kernel.base.Reflect;
import hara.kernel.flavor.NativeCapability;
import hara.kernel.flavor.NativeFlavorAccess;
import hara.kernel.flavor.NativeFlavorException;
import hara.kernel.flavor.NativeFlavorProvider;
import hara.lang.base.primitive.Array;
import hara.lang.protocol.ILookup;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.Arrays;

/** JVM implementation of the native flavor SPI. */
public final class JvmFlavorProvider implements NativeFlavorProvider {
  public static final JvmFlavorProvider INSTANCE = new JvmFlavorProvider();

  private JvmFlavorProvider() {}

  @Override
  public String name() {
    return "jvm";
  }

  @Override
  public Object resolveType(String name, NativeFlavorAccess access) {
    requireReflection(access);
    try {
      return Class.forName(name, true, access.classLoader());
    } catch (ClassNotFoundException exception) {
      if (name.indexOf('.') < 0) {
        try {
          return Class.forName("java.lang." + name, true, access.classLoader());
        } catch (ClassNotFoundException ignored) {
          // Report the original requested name below.
        }
      }
      throw unresolved("JVM class not found: " + name, exception);
    }
  }

  @Override
  public Object construct(Object type, Object[] arguments, NativeFlavorAccess access) {
    requireReflection(access);
    return invoke(
        "construct " + className(type), () -> Reflect.invokeConstructor(asClass(type), arguments));
  }

  @Override
  public Object readMember(Object target, String member, NativeFlavorAccess access) {
    requireReflection(access);
    return invoke("read JVM member " + member, () -> Reflect.getInstanceField(target, member));
  }

  @Override
  public Object writeMember(Object target, String member, Object value, NativeFlavorAccess access) {
    requireReflection(access);
    return invoke(
        "write JVM member " + member, () -> Reflect.setInstanceField(target, member, value));
  }

  @Override
  public Object invokeMember(
      Object target, String member, Object[] arguments, NativeFlavorAccess access) {
    requireReflection(access);
    return invoke(
        "invoke JVM member " + member,
        () -> Reflect.invokeInstanceMethod(target, member, arguments));
  }

  @Override
  public Object readStatic(Object type, String member, NativeFlavorAccess access) {
    requireReflection(access);
    return invoke(
        "read JVM static member " + member, () -> Reflect.getStaticField(asClass(type), member));
  }

  @Override
  public Object invokeStatic(
      Object type, String member, Object[] arguments, NativeFlavorAccess access) {
    requireReflection(access);
    return invoke(
        "invoke JVM static member " + member,
        () -> Reflect.invokeStaticMethod(asClass(type), member, arguments));
  }

  @Override
  public Object index(Object target, Object index, NativeFlavorAccess access) {
    requireReflection(access);
    if (!(index instanceof Number)) {
      throw unsupported("JVM index must be numeric");
    }
    int offset = ((Number) index).intValue();
    if (target != null && target.getClass().isArray()) {
      return java.lang.reflect.Array.get(target, offset);
    }
    if (target instanceof java.util.List<?>) {
      return ((java.util.List<?>) target).get(offset);
    }
    if (target instanceof CharSequence) {
      return ((CharSequence) target).charAt(offset);
    }
    if (target instanceof ILookup<?, ?>) {
      return ((ILookup<Object, Object>) target).lookup(index);
    }
    if (target instanceof Iterable<?>) {
      return Array.toArray((Iterable<?>) target)[offset];
    }
    throw unsupported("Value does not support JVM indexed access: " + className(target));
  }

  @Override
  public boolean matchesThrowable(Object type, Throwable throwable, NativeFlavorAccess access) {
    requireReflection(access);
    return asClass(type).isInstance(unwrap(throwable));
  }

  public Object type(Object value, NativeFlavorAccess access) {
    requireReflection(access);
    return value instanceof Class<?> ? value : value == null ? null : value.getClass();
  }

  public String typeName(Object value, NativeFlavorAccess access) {
    requireReflection(access);
    Class<?> type =
        value instanceof Class<?> ? (Class<?>) value : value == null ? null : value.getClass();
    return type == null ? "nil" : type.getName();
  }

  public boolean isInstance(Object type, Object value, NativeFlavorAccess access) {
    requireReflection(access);
    return asClass(type).isInstance(value);
  }

  public String[] fields(Object value, NativeFlavorAccess access) {
    requireReflection(access);
    return Arrays.stream(asType(value).getFields())
        .map(Field::getName)
        .distinct()
        .sorted()
        .toArray(String[]::new);
  }

  public String[] methods(Object value, NativeFlavorAccess access) {
    requireReflection(access);
    return Arrays.stream(asType(value).getMethods())
        .map(Method::getName)
        .distinct()
        .sorted()
        .toArray(String[]::new);
  }

  public String[] classPath(NativeFlavorAccess access) {
    requireCapability(access, NativeCapability.CLASSPATH, "JVM classpath");
    requireDynamicRuntime("classpath inspection");
    return access.classPath();
  }

  public String addClassPath(String location, NativeFlavorAccess access) {
    requireCapability(access, NativeCapability.CLASSPATH, "JVM classpath");
    requireDynamicRuntime("classpath mutation");
    return access.addClassPath(location);
  }

  public byte[] compile(Object expression, NativeFlavorAccess access) {
    requireCapability(access, NativeCapability.COMPILATION, "JVM compilation");
    requireDynamicRuntime("runtime compilation");
    if (!(expression instanceof hara.lang.data.List)) {
      throw unsupported("JVM compilation expects a quoted fn form");
    }
    return (byte[])
        invoke(
            "compile JVM function", () -> new Compiler().compile((hara.lang.data.List) expression));
  }

  public Class<?> defineClass(byte[] bytecode, NativeFlavorAccess access) {
    requireCapability(access, NativeCapability.COMPILATION, "JVM compilation");
    requireDynamicRuntime("runtime class definition");
    return access.defineClass(bytecode);
  }

  private static Class<?> asType(Object value) {
    if (value instanceof Class<?>) return (Class<?>) value;
    if (value != null) return value.getClass();
    throw unsupported("Expected a JVM class or value, received nil");
  }

  private static Class<?> asClass(Object value) {
    if (value instanceof Class<?>) return (Class<?>) value;
    throw unsupported("Expected a JVM class, received " + className(value));
  }

  private static void requireReflection(NativeFlavorAccess access) {
    requireCapability(access, NativeCapability.REFLECTION, "JVM reflection");
  }

  private static void requireCapability(
      NativeFlavorAccess access, NativeCapability capability, String feature) {
    if (!access.allows(capability)) {
      throw new NativeFlavorException(
          NativeFlavorException.Kind.DENIED, feature + " capability is not granted");
    }
  }

  private static void requireDynamicRuntime(String feature) {
    if (NativeMode.enabled()) {
      throw unsupported(
          "Native mode does not support "
              + feature
              + ". Use the default JVM runtime for dynamic runtime behavior.");
    }
  }

  private static Object invoke(String operation, Operation body) {
    try {
      return body.run();
    } catch (NativeFlavorException exception) {
      throw exception;
    } catch (Throwable throwable) {
      Throwable cause = unwrap(throwable);
      throw new NativeFlavorException(
          NativeFlavorException.Kind.UNSUPPORTED,
          "Unable to " + operation + ": " + cause.getMessage(),
          cause);
    }
  }

  private static Throwable unwrap(Throwable throwable) {
    Throwable value = throwable;
    while ((value instanceof InvocationTargetException
            || value instanceof java.lang.reflect.UndeclaredThrowableException)
        && value.getCause() != null) {
      value = value.getCause();
    }
    return value;
  }

  private static NativeFlavorException unsupported(String message) {
    return new NativeFlavorException(NativeFlavorException.Kind.UNSUPPORTED, message);
  }

  private static NativeFlavorException unresolved(String message, Throwable cause) {
    return new NativeFlavorException(NativeFlavorException.Kind.UNRESOLVED, message, cause);
  }

  private static String className(Object value) {
    return value == null
        ? "nil"
        : value instanceof Class<?> ? ((Class<?>) value).getName() : value.getClass().getName();
  }

  @FunctionalInterface
  private interface Operation {
    Object run() throws Throwable;
  }
}
