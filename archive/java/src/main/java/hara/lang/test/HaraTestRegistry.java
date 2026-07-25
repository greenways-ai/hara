package hara.lang.test;

import hara.lang.data.Keyword;
import hara.lang.data.types.IMapType;
import hara.truffle.HaraFunction;
import java.util.ArrayList;
import java.util.List;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.function.Predicate;
import java.util.HashSet;
import java.util.Set;

public final class HaraTestRegistry {
  @FunctionalInterface
  public interface NamespaceRunner {
    void run(String namespace, Runnable operation);
  }

  private final List<HaraTestCase> tests = new ArrayList<>();
  private final List<HaraTestFixture> fixtures = new ArrayList<>();
  private final Map<String, Object> globals = new LinkedHashMap<>();
  private final Set<String> links = new HashSet<>();
  private final Map<String, Set<String>> flags = new LinkedHashMap<>();
  private final Map<String, Map<String, Set<String>>> factFlags = new LinkedHashMap<>();

  public synchronized HaraTestCase register(
      String namespace, String name, Object metadata, HaraFunction body) {
    Object file = metadataValue(metadata, "file");
    Object line = metadataValue(metadata, "line");
    tests.removeIf(
        test ->
            namespace.equals(test.namespace())
                && name.equals(test.name())
                && sameRegistration(file, line, test.metadata()));
    HaraTestCase test = new HaraTestCase(namespace, name, metadata, body);
    tests.add(test);
    return test;
  }

  private static boolean sameRegistration(Object file, Object line, Object existingMetadata) {
    Object existingFile = metadataValue(existingMetadata, "file");
    Object existingLine = metadataValue(existingMetadata, "line");
    if (file != null && line != null && existingFile != null && existingLine != null) {
      return file.equals(existingFile) && line.equals(existingLine);
    }
    return file == null && line == null && existingFile == null && existingLine == null;
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private static Object metadataValue(Object metadata, String name) {
    if (metadata instanceof IMapType map) return map.lookup(Keyword.create(name));
    if (metadata instanceof Map map) {
      Object value = map.get(Keyword.create(name));
      return value == null ? map.get(name) : value;
    }
    return null;
  }

  public synchronized HaraTestFixture registerFixture(
      String namespace, String phase, HaraFunction body) {
    fixtures.removeIf(
        fixture -> namespace.equals(fixture.namespace()) && phase.equals(fixture.phase()));
    HaraTestFixture fixture = new HaraTestFixture(namespace, phase, body);
    fixtures.add(fixture);
    return fixture;
  }

  public synchronized List<HaraTestCase> tests() {
    return List.copyOf(tests);
  }

  public synchronized List<HaraTestFixture> fixtures() {
    return List.copyOf(fixtures);
  }

  public synchronized Map<String, Map<String, HaraTestCase>> snapshot() {
    Map<String, Map<String, HaraTestCase>> result = new LinkedHashMap<>();
    for (HaraTestCase test : tests) {
      result
          .computeIfAbsent(test.namespace(), ignored -> new LinkedHashMap<>())
          .put(test.name(), test);
    }
    return result;
  }

  public synchronized void clear() {
    tests.clear();
    fixtures.clear();
    flags.clear();
    factFlags.clear();
  }

  public synchronized void clearNamespace(String namespace) {
    tests.removeIf(test -> namespace.equals(test.namespace()));
    fixtures.removeIf(fixture -> namespace.equals(fixture.namespace()));
    factFlags.remove(namespace);
  }

  public synchronized HaraTestCase find(String namespace, String name) {
    for (HaraTestCase test : tests) {
      if (namespace.equals(test.namespace()) && name.equals(test.name())) return test;
    }
    return null;
  }

  public synchronized boolean remove(String namespace, String name) {
    factFlags.computeIfPresent(namespace, (ignored, values) -> {
      values.remove(name);
      return values;
    });
    return tests.removeIf(test -> namespace.equals(test.namespace()) && name.equals(test.name()));
  }

  public synchronized Map<String, Object> globals(String namespace) {
    Object value = globals.get(namespace);
    if (value instanceof Map<?, ?> map) {
      return new LinkedHashMap<>((Map<String, Object>) map);
    }
    return new LinkedHashMap<>();
  }

  public synchronized Map<String, Object> setGlobals(String namespace, Map<String, Object> value) {
    Map<String, Object> copy = value == null ? new LinkedHashMap<>() : new LinkedHashMap<>(value);
    globals.put(namespace, copy);
    return new LinkedHashMap<>(copy);
  }

  public synchronized Set<String> links() { return new HashSet<>(links); }
  public synchronized void addLink(String namespace) { links.add(namespace); }
  public synchronized void removeLink(String namespace) { links.remove(namespace); }

  public synchronized boolean flag(String namespace, String name) {
    return flags.getOrDefault(namespace, Set.of()).contains(name);
  }

  public synchronized boolean setFlag(String namespace, String name, boolean value) {
    Set<String> namespaceFlags = flags.computeIfAbsent(namespace, ignored -> new HashSet<>());
    if (value) namespaceFlags.add(name); else namespaceFlags.remove(name);
    return value;
  }

  public synchronized boolean factFlag(String namespace, String fact, String name) {
    return factFlags.getOrDefault(namespace, Map.of())
        .getOrDefault(fact, Set.of())
        .contains(name);
  }

  public synchronized boolean setFactFlag(
      String namespace, String fact, String name, boolean value) {
    Map<String, Set<String>> namespaceFlags =
        factFlags.computeIfAbsent(namespace, ignored -> new LinkedHashMap<>());
    Set<String> factValues = namespaceFlags.computeIfAbsent(fact, ignored -> new HashSet<>());
    if (value) factValues.add(name); else factValues.remove(name);
    return value;
  }

  public HaraTestResult run(HaraTestCase test) {
    long start = System.nanoTime();
    Object value = null;
    Throwable failure = null;
    try {
      runFixtures(test.namespace(), "before");
      value = test.body().callTarget().call(test.body().callArguments(new Object[0]));
    } catch (Throwable error) {
      failure = error;
    } finally {
      try {
        runFixtures(test.namespace(), "after");
      } catch (Throwable afterError) {
        if (failure == null) failure = afterError;
      }
    }
    return new HaraTestResult(
        test,
        failure == null ? HaraTestResult.Status.PASS : HaraTestResult.Status.FAIL,
        failure == null ? value : null,
        failure,
        elapsed(start));
  }

  public List<HaraTestResult> runAll() {
    return runAll(test -> true);
  }

  public List<HaraTestResult> runAll(Predicate<HaraTestCase> selector) {
    return runAll(selector, (namespace, operation) -> operation.run());
  }

  public List<HaraTestResult> runAll(
      Predicate<HaraTestCase> selector, NamespaceRunner namespaceRunner) {
    List<HaraTestResult> result = new ArrayList<>();
    Map<String, List<HaraTestCase>> selected = new LinkedHashMap<>();
    for (HaraTestCase test : tests()) {
      if (selector.test(test)) {
        selected.computeIfAbsent(test.namespace(), ignored -> new ArrayList<>()).add(test);
      }
    }
    for (Map.Entry<String, List<HaraTestCase>> entry : selected.entrySet()) {
      namespaceRunner.run(
          entry.getKey(),
          () -> {
            Throwable beforeAll = null;
            try {
              runFixtures(entry.getKey(), "before-all");
            } catch (Throwable error) {
              beforeAll = error;
            }
            for (HaraTestCase test : entry.getValue()) {
              result.add(beforeAll == null ? run(test) : failed(test, beforeAll));
            }
            try {
              runFixtures(entry.getKey(), "after-all");
            } catch (Throwable error) {
              if (!result.isEmpty() && beforeAll == null) {
                // Preserve one result per selected test while surfacing namespace teardown failures.
                int last = result.size() - 1;
                HaraTestResult previous = result.get(last);
                result.set(
                    last,
                    new HaraTestResult(
                        previous.test(),
                        HaraTestResult.Status.FAIL,
                        null,
                        error,
                        previous.elapsedMillis()));
              }
            }
          });
    }
    return result;
  }

  private HaraTestResult failed(HaraTestCase test, Throwable error) {
    return new HaraTestResult(test, HaraTestResult.Status.FAIL, null, error, 0L);
  }

  public void runFixtures(String namespace, String phase) {
    for (HaraTestFixture fixture : fixtures()) {
      if ((namespace == null || namespace.equals(fixture.namespace()))
          && phase.equals(fixture.phase())) {
        fixture.body().callTarget().call(fixture.body().callArguments(new Object[0]));
      }
    }
  }

  private static long elapsed(long start) {
    return (System.nanoTime() - start) / 1_000_000L;
  }
}
