package hara.truffle;

import hara.lang.base.Eq;
import hara.lang.data.Keyword;
import hara.lang.data.List;
import hara.lang.data.Symbol;
import hara.lang.data.types.ILinearType;
import hara.lang.data.types.IMapType;
import hara.lang.protocol.ICount;
import hara.lang.protocol.IMetadata;
import hara.lang.test.HaraMatcher;
import hara.lang.test.HaraTestCase;
import hara.lang.test.HaraTestRegistry;
import hara.lang.test.HaraTestResult;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.Map;

/** Static Java implementation exported exclusively as code.test/*. */
public final class CodeTest {
  private CodeTest() {}

  @HaraExport(name = "register!", arglists = {"[name function]", "[name function metadata]"})
  public static Object register(HaraContext context, Object[] values) {
    if ((values.length != 2 && values.length != 3)
        || !(values[1] instanceof HaraFunction function)) {
      throw new HaraException(
          "code.test/register! expects a name, function, and optional metadata");
    }
    context
        .testRegistry()
        .register(
            context.currentNamespaceName(),
            String.valueOf(values[0]),
            values.length == 3 ? values[2] : null,
            function);
    return null;
  }

  @HaraExport(name = "run", doc = "Runs registered facts.", arglists = {"[]", "[options]"})
  public static Object run(HaraContext context, Object[] values) {
    if (values.length > 1) {
      throw new HaraException("code.test/run accepts at most one options map");
    }
    IMapType<?, ?> options =
        values.length == 1 && values[0] instanceof IMapType<?, ?>
            ? (IMapType<?, ?>) values[0]
            : null;
    Object selector = option(options, "filter");
    Object namespaceSelector = option(options, "namespace");
    Object nameSelector = option(options, "name");
    Object metadataSelector = option(options, "metadata");
    Object onlySelector = option(options, "only");
    Object excludeSelector = option(options, "exclude");
    Object hidden = option(options, "hidden");
    if (Boolean.TRUE.equals(option(options, "clear"))) context.testRegistry().clear();
    ArrayList<Object> results = new ArrayList<>();
    for (HaraTestResult result :
        context
            .testRegistry()
            .runAll(
                test ->
                    testSelector(selector, test)
                        && testSelector(namespaceSelector, test.namespace())
                        && testSelector(nameSelector, test.name())
                        && testSelector(onlySelector, test)
                        && (excludeSelector == null || !testSelector(excludeSelector, test))
                        && metadataMatches(metadataSelector, test.metadata())
                        && (Boolean.TRUE.equals(hidden)
                            || !metadataFlag(test.metadata(), "hidden")),
                context::runInNamespace)) {
      Map<Object, Object> resultMap = new LinkedHashMap<>();
      resultMap.put("status", result.status().name());
      resultMap.put("name", result.test().name());
      resultMap.put("namespace", result.test().namespace());
      resultMap.put("elapsed", result.elapsedMillis());
      if (result.error() != null) {
        resultMap.put("error", String.valueOf(result.error().getMessage()));
      }
      if (result.test().metadata() != null) {
        resultMap.put("metadata", result.test().metadata());
      }
      results.add(resultMap);
    }
    return results;
  }

  @HaraExport(name = "run-tests", arglists = {"[]", "[options]"})
  public static Object runTests(HaraContext context, Object[] values) {
    return run(context, values);
  }

  @HaraExport(name = "tests", arglists = {"[]"})
  public static Object tests(HaraContext context, Object[] values) {
    requireArity("tests", values, 0);
    ArrayList<Object> entries = new ArrayList<>();
    for (HaraTestCase test : context.testRegistry().tests()) {
      Map<String, Object> entry = new LinkedHashMap<>();
      entry.put("namespace", test.namespace());
      entry.put("name", test.name());
      entry.put("metadata", test.metadata());
      entries.add(entry);
    }
    return entries;
  }

  @HaraExport(name = "registry", arglists = {"[]"})
  public static Object registry(HaraContext context, Object[] values) {
    requireArity("registry", values, 0);
    Map<String, Object> result = new LinkedHashMap<>();
    for (Map.Entry<String, Map<String, HaraTestCase>> namespace :
        context.testRegistry().snapshot().entrySet()) {
      Map<String, Object> facts = new LinkedHashMap<>();
      for (Map.Entry<String, HaraTestCase> fact : namespace.getValue().entrySet()) {
        facts.put(fact.getKey(), testRecord(fact.getValue()));
      }
      result.put(namespace.getKey(), facts);
    }
    return result;
  }

  @HaraExport(name = "all-facts", arglists = {"[]", "[namespace]"})
  public static Object allFacts(HaraContext context, Object[] values) {
    if (values.length > 1) throw new HaraException("all-facts accepts at most one namespace");
    String selectedNamespace = values.length == 0 ? null : namespace(values[0]);
    Map<String, Object> result = new LinkedHashMap<>();
    for (HaraTestCase test : context.testRegistry().tests()) {
      if (selectedNamespace != null && !selectedNamespace.equals(test.namespace())) continue;
      result.put(test.namespace() + "/" + test.name(), testRecord(test));
    }
    return result;
  }

  @HaraExport(name = "list-facts", arglists = {"[]", "[namespace]"})
  public static Object listFacts(HaraContext context, Object[] values) {
    if (values.length > 1) throw new HaraException("list-facts accepts at most one namespace");
    String selected =
        values.length == 0 ? context.currentNamespaceName() : namespace(values[0]);
    ArrayList<Object> result = new ArrayList<>();
    for (HaraTestCase test : context.testRegistry().tests()) {
      if (selected.equals(test.namespace())) result.add(Symbol.create(test.name()));
    }
    return result;
  }

  @HaraExport(name = "get-fact", arglists = {"[name]", "[namespace name]"})
  public static Object getFact(HaraContext context, Object[] values) {
    if (values.length < 1 || values.length > 2) {
      throw new HaraException("get-fact expects a name or namespace and name");
    }
    String namespace =
        values.length == 2 ? namespace(values[0]) : context.currentNamespaceName();
    Object name = values.length == 2 ? values[1] : values[0];
    HaraTestCase test = context.testRegistry().find(namespace, String.valueOf(name));
    return test == null ? null : testRecord(test);
  }

  @HaraExport(name = "remove-fact", arglists = {"[name]", "[namespace name]"})
  public static Object removeFact(HaraContext context, Object[] values) {
    if (values.length < 1 || values.length > 2) {
      throw new HaraException("remove-fact expects a name or namespace and name");
    }
    String namespace =
        values.length == 2 ? namespace(values[0]) : context.currentNamespaceName();
    Object name = values.length == 2 ? values[1] : values[0];
    return context.testRegistry().remove(namespace, String.valueOf(name));
  }

  @HaraExport(name = "purge-facts", arglists = {"[]", "[namespace]"})
  public static Object purgeFacts(HaraContext context, Object[] values) {
    if (values.length > 1) throw new HaraException("purge-facts accepts at most one namespace");
    String namespace =
        values.length == 0 ? context.currentNamespaceName() : namespace(values[0]);
    context.testRegistry().clearNamespace(namespace);
    return Symbol.create(namespace);
  }

  @HaraExport(name = "purge-all", arglists = {"[]"})
  public static Object purgeAll(HaraContext context, Object[] values) {
    requireArity("purge-all", values, 0);
    context.testRegistry().clear();
    return null;
  }

  @HaraExport(name = "fact-id", arglists = {"[fact]"})
  public static Object factId(HaraContext context, Object[] values) {
    requireArity("fact-id", values, 1);
    if (values[0] instanceof IMapType<?, ?> map) {
      Object id = HaraContext.lookupValue(map, Keyword.create("id"));
      if (id != null) return id;
      Object refer = HaraContext.lookupValue(map, Keyword.create("refer"));
      if (refer instanceof Symbol symbol) return Symbol.create(symbol.getName());
    }
    return values[0] instanceof Symbol symbol ? Symbol.create(symbol.getName()) : null;
  }

  @HaraExport(name = "get-global", arglists = {"[]", "[key & more]", "[namespace key & more]"})
  public static Object getGlobal(HaraContext context, Object[] values) {
    boolean firstIsKey = values.length == 1 && values[0] instanceof Keyword;
    String namespace =
        values.length == 0 || firstIsKey
            ? context.currentNamespaceName()
            : namespace(values[0]);
    Object result = context.testRegistry().globals(namespace);
    int keyStart = firstIsKey ? 0 : (values.length == 0 ? 0 : 1);
    for (int i = keyStart; i < values.length; i++) {
      if (!(result instanceof Map<?, ?> map)) return null;
      result = map.get(keyName(values[i]));
    }
    return result;
  }

  @HaraExport(name = "set-global", arglists = {"[value]", "[namespace value]"})
  public static Object setGlobal(HaraContext context, Object[] values) {
    if (values.length < 1 || values.length > 2) {
      throw new HaraException("set-global expects a value or namespace and value");
    }
    String namespace =
        values.length == 2 ? namespace(values[0]) : context.currentNamespaceName();
    Object value = values.length == 2 ? values[1] : values[0];
    return context.testRegistry().setGlobals(namespace, asStringMap(value));
  }

  @HaraExport(name = "set-flag", arglists = {"[fact flag value]", "[namespace fact flag value]"})
  public static Object setFlag(HaraContext context, Object[] values) {
    if (values.length == 3) {
      String first = namespace(values[0]);
      String flag = keyName(values[1]);
      boolean value = Boolean.TRUE.equals(values[2]);
      if (context.testRegistry().find(context.currentNamespaceName(), first) != null) {
        return context
            .testRegistry()
            .setFactFlag(context.currentNamespaceName(), first, flag, value);
      }
      return context.testRegistry().setFlag(first, flag, value);
    }
    if (values.length == 4) {
      return context
          .testRegistry()
          .setFactFlag(
              namespace(values[0]),
              namespace(values[1]),
              keyName(values[2]),
              Boolean.TRUE.equals(values[3]));
    }
    throw new HaraException(
        "set-flag expects fact, flag, value or namespace, fact, flag, value");
  }

  @HaraExport(name = "get-flag", arglists = {"[fact flag]", "[namespace fact flag]"})
  public static Object getFlag(HaraContext context, Object[] values) {
    if (values.length == 2) {
      String first = namespace(values[0]);
      String flag = keyName(values[1]);
      if (context.testRegistry().find(context.currentNamespaceName(), first) != null) {
        return context
            .testRegistry()
            .factFlag(context.currentNamespaceName(), first, flag);
      }
      return context.testRegistry().flag(first, flag);
    }
    if (values.length == 3) {
      return context
          .testRegistry()
          .factFlag(namespace(values[0]), namespace(values[1]), keyName(values[2]));
    }
    throw new HaraException("get-flag expects fact, flag or namespace, fact, flag");
  }

  @HaraExport(name = "setup-fact", arglists = {"[name]"})
  public static Object setupFact(HaraContext context, Object[] values) {
    requireArity("setup-fact", values, 1);
    HaraTestCase test =
        context
            .testRegistry()
            .find(context.currentNamespaceName(), String.valueOf(values[0]));
    if (test != null) {
      context.testRegistry().runFixtures(test.namespace(), "before");
      context
          .testRegistry()
          .setFactFlag(test.namespace(), test.name(), "setup", true);
    }
    return test == null ? null : testRecord(test);
  }

  @HaraExport(name = "teardown-fact", arglists = {"[name]"})
  public static Object teardownFact(HaraContext context, Object[] values) {
    requireArity("teardown-fact", values, 1);
    HaraTestCase test =
        context
            .testRegistry()
            .find(context.currentNamespaceName(), String.valueOf(values[0]));
    if (test != null) {
      context.testRegistry().runFixtures(test.namespace(), "after");
      context
          .testRegistry()
          .setFactFlag(test.namespace(), test.name(), "setup", false);
    }
    return test == null ? null : testRecord(test);
  }

  @HaraExport(name = "summarise", arglists = {"[results]"})
  public static Object summarise(HaraContext context, Object[] values) {
    requireArity("summarise", values, 1);
    long passed = 0;
    long failed = 0;
    long errored = 0;
    long total = 0;
    if (values[0] instanceof Iterable<?> iterable) {
      for (Object item : iterable) {
        total++;
        Object status =
            item instanceof IMapType<?, ?> map
                ? HaraContext.lookupValue(map, Keyword.create("status"))
                : item instanceof Map<?, ?> map ? map.get("status") : null;
        String name =
            status instanceof Keyword keyword ? keyword.getName() : String.valueOf(status);
        if ("pass".equalsIgnoreCase(name) || "success".equalsIgnoreCase(name)) passed++;
        else if ("fail".equalsIgnoreCase(name) || "failed".equalsIgnoreCase(name)) failed++;
        else errored++;
      }
    }
    return hara.lang.data.Map.Standard.from(
        null,
        Keyword.create("total"), total,
        Keyword.create("passed"), passed,
        Keyword.create("failed"), failed,
        Keyword.create("errored"), errored,
        Keyword.create("success"), failed == 0 && errored == 0);
  }

  @HaraExport(name = "fact:list", arglists = {"[]"})
  public static Object factList(HaraContext context, Object[] values) {
    return tests(context, values);
  }

  @HaraExport(name = "fact:get", arglists = {"[name]", "[namespace name]"})
  public static Object factGet(HaraContext context, Object[] values) {
    return getFact(context, values);
  }

  @HaraExport(name = "fact:remove", arglists = {"[name]", "[namespace name]"})
  public static Object factRemove(HaraContext context, Object[] values) {
    return removeFact(context, values);
  }

  @HaraExport(name = "fact:purge", arglists = {"[]", "[namespace]"})
  public static Object factPurge(HaraContext context, Object[] values) {
    return purgeFacts(context, values);
  }

  @HaraExport(name = "fact:setup", arglists = {"[name]"})
  public static Object factSetup(HaraContext context, Object[] values) {
    return setupFact(context, values);
  }

  @HaraExport(name = "fact:teardown", arglists = {"[name]"})
  public static Object factTeardown(HaraContext context, Object[] values) {
    return teardownFact(context, values);
  }

  @HaraExport(name = "fact:setup?", arglists = {"[fact flag]", "[namespace fact flag]"})
  public static Object factSetupPredicate(HaraContext context, Object[] values) {
    return getFlag(context, values);
  }

  @HaraExport(name = "fact:all", arglists = {"[]", "[options]"})
  public static Object factAll(HaraContext context, Object[] values) {
    return run(context, values);
  }

  @HaraExport(name = "fact:global", arglists = {"[]", "[key & more]"})
  public static Object factGlobal(HaraContext context, Object[] values) {
    return getGlobal(context, values);
  }

  @HaraExport(name = "fact:ns", arglists = {"[& namespaces]"})
  public static Object factNamespace(HaraContext context, Object[] values) {
    return values;
  }

  @HaraExport(name = "fact:ns-load", arglists = {"[namespace]"})
  public static Object factNamespaceLoad(HaraContext context, Object[] values) {
    requireArity("fact:ns-load", values, 1);
    return values[0];
  }

  @HaraExport(name = "fact:symbol", arglists = {"[value]"})
  public static Object factSymbol(HaraContext context, Object[] values) {
    requireArity("fact:symbol", values, 1);
    return values[0];
  }

  @HaraExport(name = "fact:missing", arglists = {"[]", "[namespace]"})
  public static Object factMissing(HaraContext context, Object[] values) {
    if (values.length > 1) throw new HaraException("fact:missing accepts at most one namespace");
    return hara.lang.data.Vector.Standard.EMPTY;
  }

  @HaraExport(name = "fact:exec", arglists = {"[name]"})
  public static Object factExec(HaraContext context, Object[] values) {
    if (values.length == 0) return null;
    HaraTestCase test =
        context
            .testRegistry()
            .find(context.currentNamespaceName(), String.valueOf(values[0]));
    if (test == null) return null;
    return test.body().callTarget().call(test.body().callArguments(new Object[0]));
  }

  @HaraExport(name = "capture", arglists = {"[]", "[checker]", "[checker symbol]"})
  public static Object capture(HaraContext context, Object[] values) {
    return values.length == 0 ? HaraMatcher.anything() : values[0];
  }

  @HaraExport(name = "with-new-context", arglists = {"[options & body]"})
  public static Object withNewContext(HaraContext context, Object[] values) {
    if (values.length == 0) return hara.lang.data.Map.Standard.EMPTY;
    Object body = values[values.length - 1];
    return body instanceof HaraFunction
        ? context.invokeCallable(body, new Object[0])
        : body;
  }

  @HaraExport(name = "run:interrupt", arglists = {"[]", "[options]"})
  public static Object runInterrupt(HaraContext context, Object[] values) {
    return run(context, values);
  }

  @HaraExport(name = "run:current", arglists = {"[]", "[options]"})
  public static Object runCurrent(HaraContext context, Object[] values) {
    return run(context, values);
  }

  @HaraExport(name = "run:load", arglists = {"[]", "[options]"})
  public static Object runLoad(HaraContext context, Object[] values) {
    return run(context, values);
  }

  @HaraExport(name = "run:unload", arglists = {"[]", "[options]"})
  public static Object runUnload(HaraContext context, Object[] values) {
    return run(context, values);
  }

  @HaraExport(name = "run:test", arglists = {"[]", "[options]"})
  public static Object runTest(HaraContext context, Object[] values) {
    return run(context, values);
  }

  @HaraExport(name = "run-errored", arglists = {"[]", "[options]"})
  public static Object runErrored(HaraContext context, Object[] values) {
    return run(context, values);
  }

  @HaraExport(name = "-main", arglists = {"[& args]"})
  public static Object main(HaraContext context, Object[] values) {
    return run(context, values.length == 0 ? values : new Object[0]);
  }

  @HaraExport(name = "print-options", arglists = {"[]"})
  public static Object printOptions(HaraContext context, Object[] values) {
    requireArity("print-options", values, 0);
    return hara.lang.data.Set.Standard.from(
        null,
        Keyword.create("help"),
        Keyword.create("current"),
        Keyword.create("default"),
        Keyword.create("disable"),
        Keyword.create("all"));
  }

  @HaraExport(name = "=>", kind = HaraExport.Kind.VALUE)
  public static Object arrow(HaraContext context) {
    return Symbol.create("=>");
  }

  @HaraExport(name = "assert!", arglists = {"[actual expected]"})
  public static Object assertValue(HaraContext context, Object[] values) {
    requireArity("assert!", values, 2);
    Object actual = HaraBox.unwrap(values[0]);
    Object expected = HaraBox.unwrap(values[1]);
    boolean matched;
    if (expected instanceof HaraMatcher matcher) matched = matcher.matches(actual);
    else if (context.isFunctionValue(expected)) {
      matched =
          Boolean.TRUE.equals(
              HaraBox.unwrap(context.invokeCallable(expected, new Object[] {actual})));
    } else if (expected instanceof Class<?> type) {
      matched = actual != null && type.isInstance(actual);
    } else matched = Eq.eq(actual, expected);
    if (!matched) {
      throw new HaraException(
          "Assertion failed: expected " + values[1] + ", received " + values[0]);
    }
    return true;
  }

  @HaraExport(name = "assert-throws!", arglists = {"[function matcher]"})
  public static Object assertThrows(HaraContext context, Object[] values) {
    if (values.length != 2 || !(values[0] instanceof HaraFunction function)) {
      throw new HaraException(
          "code.test/assert-throws! expects a function and matcher");
    }
    try {
      function.callTarget().call(function.callArguments(new Object[0]));
    } catch (Throwable error) {
      Object matcher = HaraBox.unwrap(values[1]);
      if (matcher instanceof HaraMatcher haraMatcher && haraMatcher.matches(error)) return true;
      if (matcher instanceof HaraFunction predicate
          && Boolean.TRUE.equals(
              predicate
                  .callTarget()
                  .call(predicate.callArguments(new Object[] {error})))) return true;
      if (matcher instanceof Class<?> type && type.isInstance(error)) return true;
      if (matcher instanceof HaraMatcher haraMatcher
          && haraMatcher.kind() == HaraMatcher.Kind.THROWS) return true;
      throw new HaraException("Unexpected exception: " + error.getMessage());
    }
    throw new HaraException("Expected expression to throw");
  }

  @HaraExport(name = "checker?", arglists = {"[value]"})
  public static Object checkerPredicate(HaraContext context, Object[] values) {
    requireArity("checker?", values, 1);
    return values[0] instanceof HaraMatcher;
  }

  @HaraExport(name = "->checker", arglists = {"[value]"})
  public static Object toChecker(HaraContext context, Object[] values) {
    requireArity("->checker", values, 1);
    return values[0] instanceof HaraMatcher
        ? values[0]
        : HaraMatcher.satisfies(values[0]);
  }

  @HaraExport(name = "verify", arglists = {"[checker value]"})
  public static Object verify(HaraContext context, Object[] values) {
    if (values.length != 2 || !(values[0] instanceof HaraMatcher checker)) {
      throw new HaraException("verify expects a checker and value");
    }
    boolean matched;
    Throwable error = null;
    try {
      matched = checker.matches(HaraBox.unwrap(values[1]));
    } catch (Throwable throwable) {
      matched = false;
      error = throwable;
    }
    return hara.lang.data.Map.Standard.from(
        null,
        Keyword.create("status"),
        error == null ? Keyword.create("success") : Keyword.create("exception"),
        Keyword.create("data"),
        error == null ? matched : error,
        Keyword.create("checker"),
        checker,
        Keyword.create("actual"),
        values[1],
        Keyword.create("from"),
        Keyword.create("verify"));
  }

  @HaraExport(name = "succeeded?", arglists = {"[result]"})
  public static Object succeededPredicate(HaraContext context, Object[] values) {
    requireArity("succeeded?", values, 1);
    Object status;
    Object data;
    if (values[0] instanceof IMapType<?, ?> map) {
      status = HaraContext.lookupValue(map, Keyword.create("status"));
      data = HaraContext.lookupValue(map, Keyword.create("data"));
    } else if (values[0] instanceof Map<?, ?> map) {
      status = map.get("status");
      data = map.get("data");
    } else return false;
    return (status instanceof Keyword keyword && "success".equals(keyword.getName())
            || "success".equals(String.valueOf(status)))
        && Boolean.TRUE.equals(data);
  }

  @HaraExport(name = "anything", kind = HaraExport.Kind.VALUE)
  public static Object anything(HaraContext context) {
    return HaraMatcher.anything();
  }

  @HaraExport(name = "contains", arglists = {"[expected & modifiers]"})
  public static Object contains(HaraContext context, Object[] values) {
    requireNonEmpty("contains", values);
    return HaraMatcher.contains(
        values[0], matcherOption(values, "in-any-order"), matcherOption(values, "gaps-ok"));
  }

  @HaraExport(name = "contains-in", arglists = {"[expected & modifiers]"})
  public static Object containsIn(HaraContext context, Object[] values) {
    requireNonEmpty("contains-in", values);
    return HaraMatcher.containsIn(
        values[0], matcherOption(values, "in-any-order"), matcherOption(values, "gaps-ok"));
  }

  @HaraExport(name = "just", arglists = {"[expected & modifiers]"})
  public static Object just(HaraContext context, Object[] values) {
    requireNonEmpty("just", values);
    return HaraMatcher.just(
        values[0], matcherOption(values, "in-any-order"), matcherOption(values, "gaps-ok"));
  }

  @HaraExport(name = "just-in", arglists = {"[expected]"})
  public static Object justIn(HaraContext context, Object[] values) {
    requireArity("just-in", values, 1);
    return HaraMatcher.justIn(values[0]);
  }

  @HaraExport(name = "exactly", arglists = {"[value]", "[value projection]"})
  public static Object exactly(HaraContext context, Object[] values) {
    if (values.length < 1 || values.length > 2) {
      throw new HaraException("exactly expects a value and optional projection");
    }
    return HaraMatcher.exactly(values[0]);
  }

  @HaraExport(name = "approx", arglists = {"[value]", "[value threshold]"})
  public static Object approx(HaraContext context, Object[] values) {
    if (values.length < 1
        || values.length > 2
        || !(values[0] instanceof Number)) {
      throw new HaraException("approx expects a numeric value and optional threshold");
    }
    double threshold =
        values.length == 2 ? ((Number) values[1]).doubleValue() : 0.001d;
    return HaraMatcher.approximate(values[0], threshold);
  }

  @HaraExport(name = "satisfies", arglists = {"[expected]"})
  public static Object satisfies(HaraContext context, Object[] values) {
    requireArity("satisfies", values, 1);
    return HaraMatcher.satisfies(values[0]);
  }

  @HaraExport(name = "stores", arglists = {"[expected]"})
  public static Object stores(HaraContext context, Object[] values) {
    requireArity("stores", values, 1);
    return HaraMatcher.stores(values[0]);
  }

  @HaraExport(name = "any", arglists = {"[& checkers]"})
  public static Object any(HaraContext context, Object[] values) {
    return HaraMatcher.any(values);
  }

  @HaraExport(name = "all", arglists = {"[& checkers]"})
  public static Object all(HaraContext context, Object[] values) {
    return HaraMatcher.all(values);
  }

  @HaraExport(name = "is-not", arglists = {"[checker]"})
  public static Object isNot(HaraContext context, Object[] values) {
    requireArity("is-not", values, 1);
    return HaraMatcher.isNot(values[0]);
  }

  @HaraExport(name = "is", arglists = {"[predicate]"})
  public static Object is(HaraContext context, Object[] values) {
    requireArity("is", values, 1);
    return HaraMatcher.predicate(values[0]);
  }

  @HaraExport(name = "nil?", arglists = {"[value]"})
  public static Object nilPredicate(HaraContext context, Object[] values) {
    requireArity("nil?", values, 1);
    return values[0] == null;
  }

  @HaraExport(name = "string?", arglists = {"[value]"})
  public static Object stringPredicate(HaraContext context, Object[] values) {
    requireArity("string?", values, 1);
    return values[0] instanceof String;
  }

  @HaraExport(name = "number?", arglists = {"[value]"})
  public static Object numberPredicate(HaraContext context, Object[] values) {
    requireArity("number?", values, 1);
    return values[0] instanceof Number;
  }

  @HaraExport(name = "integer?", arglists = {"[value]"})
  public static Object integerPredicate(HaraContext context, Object[] values) {
    requireArity("integer?", values, 1);
    return values[0] instanceof Byte
        || values[0] instanceof Short
        || values[0] instanceof Integer
        || values[0] instanceof Long;
  }

  @HaraExport(name = "boolean?", arglists = {"[value]"})
  public static Object booleanPredicate(HaraContext context, Object[] values) {
    requireArity("boolean?", values, 1);
    return values[0] instanceof Boolean;
  }

  @HaraExport(name = "map?", arglists = {"[value]"})
  public static Object mapPredicate(HaraContext context, Object[] values) {
    requireArity("map?", values, 1);
    return values[0] instanceof IMapType<?, ?> || values[0] instanceof Map<?, ?>;
  }

  @HaraExport(name = "vector?", arglists = {"[value]"})
  public static Object vectorPredicate(HaraContext context, Object[] values) {
    requireArity("vector?", values, 1);
    return values[0] instanceof hara.lang.data.Vector<?>;
  }

  @HaraExport(name = "seq?", arglists = {"[value]"})
  public static Object sequencePredicate(HaraContext context, Object[] values) {
    requireArity("seq?", values, 1);
    return values[0] instanceof ILinearType<?>;
  }

  @HaraExport(name = "coll?", arglists = {"[value]"})
  public static Object collectionPredicate(HaraContext context, Object[] values) {
    requireArity("coll?", values, 1);
    return values[0] instanceof ILinearType<?>
        || values[0] instanceof IMapType<?, ?>
        || values[0] instanceof Iterable<?>;
  }

  @HaraExport(name = "fn?", arglists = {"[value]"})
  public static Object functionPredicate(HaraContext context, Object[] values) {
    requireArity("fn?", values, 1);
    return values[0] instanceof HaraFunction;
  }

  @HaraExport(name = "var?", arglists = {"[value]"})
  public static Object varPredicate(HaraContext context, Object[] values) {
    requireArity("var?", values, 1);
    return values[0] instanceof HaraVar;
  }

  @HaraExport(name = "any?", arglists = {"[value]"})
  public static Object anyPredicate(HaraContext context, Object[] values) {
    requireArity("any?", values, 1);
    return values[0] instanceof Iterable<?> iterable && iterable.iterator().hasNext();
  }

  @HaraExport(name = "empty?", arglists = {"[value]"})
  public static Object emptyPredicate(HaraContext context, Object[] values) {
    requireArity("empty?", values, 1);
    Object value = values[0];
    if (value == null) return true;
    if (value instanceof ICount count) return count.count() == 0;
    return value instanceof Iterable<?> iterable && !iterable.iterator().hasNext();
  }

  @HaraExport(name = "throws", arglists = {"[]", "[predicate]"})
  public static Object throwsMatcher(HaraContext context, Object[] values) {
    if (values.length > 1) throw new HaraException("throws expects an optional predicate");
    return HaraMatcher.throwsMatcher(values.length == 0 ? null : values[0]);
  }

  @HaraExport(name = "throws-info", arglists = {"[]", "[data]"})
  public static Object throwsInfo(HaraContext context, Object[] values) {
    if (values.length > 1) throw new HaraException("throws-info expects an optional map");
    return HaraMatcher.throwsInfo(values.length == 0 ? null : values[0]);
  }

  @HaraExport(name = "use-fixtures", arglists = {"[phase function]"})
  public static Object useFixtures(HaraContext context, Object[] values) {
    if (values.length != 2
        || !(values[0] instanceof Keyword keyword)
        || !(values[1] instanceof HaraFunction function)) {
      throw new HaraException(
          "code.test/use-fixtures expects a phase keyword and function");
    }
    String phase = keyword.getName();
    if (!phase.equals("before")
        && !phase.equals("after")
        && !phase.equals("before-all")
        && !phase.equals("after-all")) {
      throw new HaraException(
          "code.test/use-fixtures phase must be :before, :after, :before-all, or :after-all");
    }
    context
        .testRegistry()
        .registerFixture(context.currentNamespaceName(), phase, function);
    return null;
  }

  @HaraExport(name = "clear!", arglists = {"[]", "[namespace]"})
  public static Object clear(HaraContext context, Object[] values) {
    if (values.length == 0) context.testRegistry().clear();
    else if (values.length == 1) context.testRegistry().clearNamespace(namespace(values[0]));
    else throw new HaraException("code.test/clear! expects no arguments or a namespace");
    return null;
  }

  @HaraExport(
      name = "fact",
      doc = "Defines and registers a test fact.",
      arglists = {"[description & body]"},
      kind = HaraExport.Kind.MACRO,
      intrinsic = true)
  public static Object fact(HaraContext context, List invocation) {
    return expandFact(invocation);
  }

  @HaraExport(
      name = "fact:template",
      arglists = {"[description & body]"},
      kind = HaraExport.Kind.MACRO)
  public static Object factTemplate(HaraContext context, List invocation) {
    return expandFact(invocation);
  }

  private static Object expandFact(List<?> invocation) {
    if (invocation.count() < 2) throw new HaraException("fact expects a name");
    Object name = invocation.nth(1);
    ArrayList<Object> body = new ArrayList<>();
    for (int i = 2; i < invocation.count(); i++) {
      Object form = invocation.nth(i);
      if (i + 2 < invocation.count()
          && form instanceof Symbol symbol
          && "=>".equals(symbol.getName())) {
        throw new HaraException("fact assertion is missing an actual expression");
      }
      if (i + 2 < invocation.count()
          && invocation.nth(i + 1) instanceof Symbol arrow
          && "=>".equals(arrow.getName())) {
        Object expected = invocation.nth(i + 2);
        boolean bareThrows =
            expected instanceof Symbol expectedSymbol
                && expectedSymbol.getNamespace() == null
                && "throws".equals(expectedSymbol.getName());
        boolean throwsForm =
            expected instanceof List<?> expectedForm
                && expectedForm.count() > 0
                && expectedForm.nth(0) instanceof Symbol operator
                && "throws".equals(operator.getName());
        if (bareThrows || throwsForm) {
          Object matcher =
              bareThrows
                  ? List.Standard.from(null, Symbol.create("code.test", "throws"))
                  : expected;
          body.add(
              List.Standard.from(
                  null,
                  Symbol.create("code.test", "assert-throws!"),
                  List.Standard.from(
                      null,
                      Symbol.create("fn"),
                      hara.lang.data.Vector.Standard.EMPTY,
                      form),
                  matcher));
        } else {
          body.add(
              List.Standard.from(
                  null, Symbol.create("code.test", "assert!"), form, expected));
        }
        i += 2;
      } else body.add(form);
    }
    Object[] function;
    if (body.size() == 1) {
      function =
          new Object[] {
            Symbol.create("fn"), hara.lang.data.Vector.Standard.EMPTY, body.get(0)
          };
    } else {
      function =
          new Object[] {
            Symbol.create("fn"),
            hara.lang.data.Vector.Standard.EMPTY,
            List.Standard.from(null, prepend(Symbol.create("do"), body))
          };
    }
    IMetadata metadata =
        invocation instanceof hara.lang.protocol.IObjType object ? object.meta() : null;
    if (metadata != null) {
      return List.Standard.from(
          null,
          Symbol.create("code.test", "register!"),
          name,
          List.Standard.from(null, function),
          metadata);
    }
    return List.Standard.from(
        null,
        Symbol.create("code.test", "register!"),
        name,
        List.Standard.from(null, function));
  }

  private static Object option(IMapType<?, ?> options, String name) {
    return options == null ? null : HaraContext.lookupValue(options, Keyword.create(name));
  }

  private static boolean testSelector(Object selector, HaraTestCase test) {
    return selector == null
        || testSelector(selector, test.namespace() + "/" + test.name());
  }

  private static boolean testSelector(Object selector, String id) {
    if (selector == null) return true;
    if (selector instanceof java.util.function.Predicate<?> predicate) {
      @SuppressWarnings("unchecked")
      java.util.function.Predicate<Object> test =
          (java.util.function.Predicate<Object>) predicate;
      return test.test(id);
    }
    if (selector instanceof HaraFunction function) {
      return Boolean.TRUE.equals(
          function.callTarget().call(function.callArguments(new Object[] {id})));
    }
    if (selector instanceof java.util.regex.Pattern pattern) {
      return pattern.matcher(id).find();
    }
    if (selector instanceof hara.lang.data.types.ISetType<?> set) {
      @SuppressWarnings("rawtypes")
      hara.lang.data.types.ISetType rawSet = (hara.lang.data.types.ISetType) set;
      return rawSet.find(id) != null;
    }
    if (selector instanceof hara.lang.data.Vector<?> vector) {
      for (Object item : vector) if (testSelector(item, id)) return true;
      return false;
    }
    if (selector instanceof hara.lang.data.List<?> list) {
      for (Object item : list) if (!testSelector(item, id)) return false;
      return true;
    }
    if (selector instanceof String string) return id.startsWith(string);
    if (selector instanceof Symbol symbol) return id.startsWith(symbol.display());
    if (selector instanceof Keyword keyword) return id.startsWith(keyword.getName());
    return false;
  }

  private static boolean metadataMatches(Object expected, Object metadata) {
    if (expected == null) return true;
    if (expected instanceof HaraMatcher matcher) return matcher.matches(metadata);
    if (expected instanceof HaraFunction function) {
      return Boolean.TRUE.equals(
          function.callTarget().call(function.callArguments(new Object[] {metadata})));
    }
    if (expected instanceof IMapType<?, ?> expectedMap
        && metadata instanceof IMapType<?, ?> actualMap) {
      for (Object object : expectedMap) {
        Map.Entry<?, ?> entry = (Map.Entry<?, ?>) object;
        Object actual = HaraContext.lookupValue(actualMap, entry.getKey());
        if (actual == null || !Eq.eq(actual, entry.getValue())) return false;
      }
      return true;
    }
    if (expected instanceof Map<?, ?> expectedMap && metadata instanceof Map<?, ?> actualMap) {
      for (Map.Entry<?, ?> entry : expectedMap.entrySet()) {
        if (!actualMap.containsKey(entry.getKey())
            || !Eq.eq(actualMap.get(entry.getKey()), entry.getValue())) return false;
      }
      return true;
    }
    return Eq.eq(expected, metadata);
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private static boolean metadataFlag(Object metadata, String name) {
    return metadata instanceof IMapType<?, ?> map
        && Boolean.TRUE.equals(((IMapType) map).lookup(Keyword.create(name)));
  }

  private static Map<String, Object> testRecord(HaraTestCase test) {
    Map<String, Object> record = new LinkedHashMap<>();
    record.put("namespace", test.namespace());
    record.put("name", test.name());
    record.put("metadata", test.metadata());
    return record;
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private static Map<String, Object> asStringMap(Object value) {
    Map<String, Object> result = new LinkedHashMap<>();
    if (value instanceof Map<?, ?> map) {
      map.forEach((key, nested) -> result.put(keyName(key), nested));
    } else if (value instanceof IMapType<?, ?> map) {
      for (Object object : map) {
        Map.Entry entry = (Map.Entry) object;
        result.put(keyName(entry.getKey()), entry.getValue());
      }
    }
    return result;
  }

  private static boolean matcherOption(Object[] values, String name) {
    for (int i = 1; i < values.length; i++) {
      if (values[i] instanceof Keyword keyword && name.equals(keyword.getName())) return true;
      if (values[i] instanceof Symbol symbol && name.equals(symbol.getName())) return true;
    }
    return false;
  }

  private static String namespace(Object value) {
    return value instanceof Symbol symbol ? symbol.display() : String.valueOf(value);
  }

  private static String keyName(Object key) {
    return key instanceof Keyword keyword
        ? keyword.getName()
        : String.valueOf(key).replaceFirst("^:", "");
  }

  private static Object[] prepend(Object first, java.util.List<Object> rest) {
    Object[] values = new Object[rest.size() + 1];
    values[0] = first;
    for (int i = 0; i < rest.size(); i++) values[i + 1] = rest.get(i);
    return values;
  }

  private static void requireArity(String operation, Object[] values, int expected) {
    if (values.length != expected) {
      throw new HaraException(operation + " expects " + expected + " arguments");
    }
  }

  private static void requireNonEmpty(String operation, Object[] values) {
    if (values.length == 0) throw new HaraException(operation + " expects a value");
  }
}
