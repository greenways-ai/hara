package hara.kernel.flavor;

/**
 * Evaluator-neutral extension point for platform-specific language behavior.
 *
 * <p>The core evaluators own syntax, evaluation order, namespace state, and authority. Providers
 * only adapt already-evaluated values.
 */
public interface NativeFlavorProvider {
  String name();

  Object resolveType(String name, NativeFlavorAccess access);

  Object construct(Object type, Object[] arguments, NativeFlavorAccess access);

  Object readMember(Object target, String member, NativeFlavorAccess access);

  Object writeMember(Object target, String member, Object value, NativeFlavorAccess access);

  Object invokeMember(Object target, String member, Object[] arguments, NativeFlavorAccess access);

  Object readStatic(Object type, String member, NativeFlavorAccess access);

  Object invokeStatic(Object type, String member, Object[] arguments, NativeFlavorAccess access);

  Object index(Object target, Object index, NativeFlavorAccess access);

  boolean matchesThrowable(Object type, Throwable throwable, NativeFlavorAccess access);
}
