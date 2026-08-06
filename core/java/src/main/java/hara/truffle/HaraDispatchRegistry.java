package hara.truffle;

import com.oracle.truffle.api.Assumption;
import com.oracle.truffle.api.Truffle;
import java.util.ArrayDeque;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

/** Context-local dispatch tables shared by the methods of one protocol descriptor. */
public final class HaraDispatchRegistry {
  private final Map<String, Table> tables = new ConcurrentHashMap<>();
  private volatile Assumption stable = Truffle.getRuntime().createAssumption();

  public void register(
      String method, HaraDispatchKey key, HaraProtocolImplementation implementation) {
    tables.computeIfAbsent(method, ignored -> new Table()).register(key, implementation);
    Assumption previous = stable;
    previous.invalidate();
    stable = Truffle.getRuntime().createAssumption();
  }

  public HaraProtocolImplementation resolve(String method, Object receiver) {
    Table table = tables.get(method);
    return table == null ? null : table.resolve(receiver);
  }

  public Assumption stable() {
    return stable;
  }

  private static final class Table {
    private static final HaraProtocolImplementation NO_IMPLEMENTATION =
        new HaraProtocolImplementation(null, null);

    private final Map<HaraType, HaraProtocolImplementation> haraTypes = new ConcurrentHashMap<>();
    private final Map<Class<?>, HaraProtocolImplementation> javaClasses = new ConcurrentHashMap<>();
    private final Map<HaraDispatchKey.PrimitiveCategory, HaraProtocolImplementation> primitives =
        new ConcurrentHashMap<>();
    private final Map<Class<?>, HaraProtocolImplementation> resolvedClasses =
        new ConcurrentHashMap<>();
    private volatile HaraProtocolImplementation nil;
    private volatile HaraProtocolImplementation foreign;
    private volatile HaraProtocolImplementation defaultImplementation;

    private void register(HaraDispatchKey key, HaraProtocolImplementation implementation) {
      switch (key.kind()) {
        case HARA_TYPE:
          haraTypes.put(key.haraType(), implementation);
          break;
        case JAVA_CLASS:
          javaClasses.put(key.javaClass(), implementation);
          break;
        case PRIMITIVE:
          primitives.put(key.primitiveCategory(), implementation);
          break;
        case NIL:
          nil = implementation;
          break;
        case FOREIGN:
          foreign = implementation;
          break;
        case DEFAULT:
          defaultImplementation = implementation;
          break;
        default:
          throw new AssertionError(key.kind());
      }
      resolvedClasses.clear();
    }

    private HaraProtocolImplementation resolve(Object receiver) {
      if (receiver == null) {
        return nil != null ? nil : defaultImplementation;
      }

      if (receiver instanceof HaraStruct) {
        HaraProtocolImplementation implementation = haraTypes.get(((HaraStruct) receiver).type());
        if (implementation != null) {
          return implementation;
        }
      }

      if (HaraDispatchKey.isForeign(receiver) && foreign != null) {
        return foreign;
      }

      Class<?> receiverClass = receiver.getClass();
      HaraProtocolImplementation cached = resolvedClasses.get(receiverClass);
      if (cached != null) {
        return cached == NO_IMPLEMENTATION ? null : cached;
      }
      HaraProtocolImplementation implementation = resolveClass(receiverClass, receiver);
      resolvedClasses.put(
          receiverClass, implementation == null ? NO_IMPLEMENTATION : implementation);
      return implementation;
    }

    private HaraProtocolImplementation resolveClass(Class<?> receiverClass, Object receiver) {
      HaraProtocolImplementation implementation = javaClasses.get(receiverClass);
      if (implementation != null) {
        return implementation;
      }
      implementation = mostSpecificJavaClass(receiverClass);
      if (implementation != null) {
        return implementation;
      }

      HaraDispatchKey.PrimitiveCategory category = HaraDispatchKey.primitiveCategory(receiver);
      implementation = category == null ? null : primitives.get(category);
      return implementation == null ? defaultImplementation : implementation;
    }

    private HaraProtocolImplementation mostSpecificJavaClass(Class<?> receiverClass) {
      HaraProtocolImplementation result = null;
      int bestDistance = Integer.MAX_VALUE;
      for (Map.Entry<Class<?>, HaraProtocolImplementation> entry : javaClasses.entrySet()) {
        Class<?> registeredClass = entry.getKey();
        if (!registeredClass.isAssignableFrom(receiverClass)) {
          continue;
        }
        int distance = inheritanceDistance(receiverClass, registeredClass);
        if (distance < bestDistance) {
          bestDistance = distance;
          result = entry.getValue();
        }
      }
      return result;
    }

    private int inheritanceDistance(Class<?> receiverClass, Class<?> registeredClass) {
      ArrayDeque<Class<?>> queue = new ArrayDeque<>();
      Map<Class<?>, Integer> distances = new ConcurrentHashMap<>();
      queue.add(receiverClass);
      distances.put(receiverClass, 0);
      while (!queue.isEmpty()) {
        Class<?> current = queue.removeFirst();
        int distance = distances.get(current);
        if (current == registeredClass) {
          return distance;
        }
        Class<?> superclass = current.getSuperclass();
        if (superclass != null && distances.putIfAbsent(superclass, distance + 1) == null) {
          queue.addLast(superclass);
        }
        for (Class<?> interfaceType : current.getInterfaces()) {
          if (distances.putIfAbsent(interfaceType, distance + 1) == null) {
            queue.addLast(interfaceType);
          }
        }
      }
      return Integer.MAX_VALUE;
    }
  }
}
