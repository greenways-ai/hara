package hara.lang.test;

import hara.lang.base.Eq;
import hara.lang.data.types.IMapType;
import hara.lang.data.types.ILinearType;
import hara.lang.protocol.IDeref;
import hara.truffle.HaraFunction;
import java.util.Iterator;
import java.util.Map;
import java.util.Objects;
import java.util.function.Predicate;

/** Matchers used by the Java-backed code.test runtime. */
public final class HaraMatcher {
  public enum Kind {
    ANYTHING, PREDICATE, EXACTLY, APPROX, CONTAINS, CONTAINS_IN, JUST, JUST_IN, THROWS, THROWS_INFO,
    STORES, ANY, ALL, IS_NOT
  }

  private final Kind kind;
  private final Object expected;
  private final boolean inAnyOrder;
  private final boolean gapsOk;
  private final double threshold;

  private HaraMatcher(Kind kind, Object expected) {
    this(kind, expected, false, false, 0.0d);
  }

  private HaraMatcher(Kind kind, Object expected, boolean inAnyOrder, boolean gapsOk) {
    this(kind, expected, inAnyOrder, gapsOk, 0.0d);
  }

  private HaraMatcher(Kind kind, Object expected, boolean inAnyOrder, boolean gapsOk, double threshold) {
    this.kind = kind;
    this.expected = expected;
    this.inAnyOrder = inAnyOrder;
    this.gapsOk = gapsOk;
    this.threshold = threshold;
  }

  public static HaraMatcher anything() { return new HaraMatcher(Kind.ANYTHING, null); }
  public static HaraMatcher predicate(Object value) { return new HaraMatcher(Kind.PREDICATE, value); }
  public static HaraMatcher contains(Object value) { return new HaraMatcher(Kind.CONTAINS, value); }
  public static HaraMatcher containsIn(Object value) { return new HaraMatcher(Kind.CONTAINS_IN, value); }
  public static HaraMatcher contains(Object value, boolean inAnyOrder, boolean gapsOk) {
    return new HaraMatcher(Kind.CONTAINS, value, inAnyOrder, gapsOk);
  }
  public static HaraMatcher containsIn(Object value, boolean inAnyOrder, boolean gapsOk) {
    return new HaraMatcher(Kind.CONTAINS_IN, value, inAnyOrder, gapsOk);
  }
  public static HaraMatcher just(Object value) { return new HaraMatcher(Kind.JUST, value); }
  public static HaraMatcher justIn(Object value) { return new HaraMatcher(Kind.JUST_IN, value); }
  public static HaraMatcher just(Object value, boolean inAnyOrder, boolean gapsOk) {
    return new HaraMatcher(Kind.JUST, value, inAnyOrder, gapsOk);
  }
  public static HaraMatcher exactly(Object value) { return new HaraMatcher(Kind.EXACTLY, value); }
  public static HaraMatcher approximate(Object value, double threshold) {
    return new HaraMatcher(Kind.APPROX, value, false, false, threshold);
  }
  public static HaraMatcher satisfies(Object value) { return new HaraMatcher(Kind.PREDICATE, value); }
  public static HaraMatcher stores(Object value) { return new HaraMatcher(Kind.STORES, value); }
  public static HaraMatcher any(Object... values) { return new HaraMatcher(Kind.ANY, values); }
  public static HaraMatcher all(Object... values) { return new HaraMatcher(Kind.ALL, values); }
  public static HaraMatcher isNot(Object value) { return new HaraMatcher(Kind.IS_NOT, value); }
  public static HaraMatcher throwsMatcher(Object value) { return new HaraMatcher(Kind.THROWS, value); }
  public static HaraMatcher throwsInfo(Object value) { return new HaraMatcher(Kind.THROWS_INFO, value); }

  public Kind kind() { return kind; }
  public Object expected() { return expected; }

  public boolean matches(Object actual) {
    if (kind == Kind.ANYTHING) return true;
    if (kind == Kind.PREDICATE || kind == Kind.STORES) {
      if (kind == Kind.STORES && actual instanceof IDeref<?> ref) {
        return matchesExpected(ref.deref(), expected, true);
      }
      if (kind == Kind.STORES && actual instanceof java.util.concurrent.atomic.AtomicBoolean ref) {
        return matchesExpected(ref.get(), expected, true);
      }
      return predicateMatches(expected, actual);
    }
    if (kind == Kind.EXACTLY) return Eq.eq(actual, expected);
    if (kind == Kind.APPROX) {
      return actual instanceof Number && expected instanceof Number
          && Math.abs(((Number) actual).doubleValue() - ((Number) expected).doubleValue()) < threshold;
    }
    if (kind == Kind.THROWS || kind == Kind.THROWS_INFO) {
      if (!(actual instanceof Throwable)) return false;
      if (expected == null) return true;
      return predicateMatches(expected, actual) || matchesExpected(actual, expected, true);
    }
    if (kind == Kind.ANY || kind == Kind.ALL) {
      Object[] values = expected instanceof Object[] array ? array : new Object[] {expected};
      boolean result = kind == Kind.ALL;
      for (Object value : values) {
        boolean matched = matchesExpected(actual, value, true);
        result = kind == Kind.ALL ? result && matched : result || matched;
      }
      return result;
    }
    if (kind == Kind.IS_NOT) return !matchesExpected(actual, expected, true);
    if (kind == Kind.CONTAINS || kind == Kind.CONTAINS_IN || kind == Kind.JUST || kind == Kind.JUST_IN) {
      boolean nested = kind == Kind.CONTAINS_IN || kind == Kind.JUST_IN;
      boolean exact = kind == Kind.JUST || kind == Kind.JUST_IN;
      return containsMatches(actual, expected, nested, exact, inAnyOrder, gapsOk);
    }
    return false;
  }

  private static boolean predicateMatches(Object predicate, Object value) {
    if (predicate instanceof Predicate<?> p) {
      @SuppressWarnings("unchecked") Predicate<Object> test = (Predicate<Object>) p;
      return test.test(value);
    }
    if (predicate instanceof HaraFunction function) {
      Object result = function.callTarget().call(function.callArguments(new Object[] {value}));
      return Boolean.TRUE.equals(result);
    }
    if (predicate instanceof Class<?> type) return value != null && type.isInstance(value);
    if (predicate instanceof java.util.regex.Pattern pattern) {
      return pattern.matcher(String.valueOf(value)).find();
    }
    return Eq.eq(value, predicate);
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private static boolean containsMatches(
      Object actual, Object expected, boolean nested, boolean exact, boolean inAnyOrder, boolean gapsOk) {
    if (expected instanceof IMapType expectedMap) {
      if (actual instanceof IMapType actualMap) {
        if (exact && expectedMap.count() != actualMap.count()) return false;
        for (Object entryObject : expectedMap) {
          Map.Entry entry = (Map.Entry) entryObject;
          Object actualValue = actualMap.lookup(entry.getKey());
          if (actualValue == null && actualMap.find(entry.getKey()) == null) return false;
          if (!matchesExpected(actualValue, entry.getValue(), nested)) return false;
        }
        return true;
      }
      if (actual instanceof Map actualMap) {
        if (exact && expectedMap.count() != actualMap.size()) return false;
        for (Object entryObject : expectedMap) {
          Map.Entry entry = (Map.Entry) entryObject;
          if (!actualMap.containsKey(entry.getKey())
              || !matchesExpected(actualMap.get(entry.getKey()), entry.getValue(), nested)) return false;
        }
        return true;
      }
      return false;
    }
    if (expected instanceof ILinearType expectedLinear) {
      if (!(actual instanceof Iterable<?>)) return false;
      java.util.List<Object> actualValues = new java.util.ArrayList<>();
      for (Object value : (Iterable<?>) actual) actualValues.add(value);
      java.util.List<Object> wantedValues = new java.util.ArrayList<>();
      for (Object value : expectedLinear) wantedValues.add(value);
      if (inAnyOrder) {
        for (Object wanted : wantedValues) {
          boolean found = false;
          for (Object value : actualValues) if (matchesExpected(value, wanted, nested)) { found = true; break; }
          if (!found) return false;
        }
        return !exact || actualValues.size() == wantedValues.size();
      }
      int index = 0;
      for (Object wanted : wantedValues) {
        boolean found = false;
        while (index < actualValues.size()) {
          if (matchesExpected(actualValues.get(index++), wanted, nested)) { found = true; break; }
          if (!gapsOk) return false;
        }
        if (!found) return false;
      }
      return !exact || actualValues.size() == wantedValues.size();
    }
    if (expected instanceof Iterable<?> expectedValues && actual instanceof Iterable<?> actualValues) {
      for (Object wanted : expectedValues) {
        boolean found = false;
        for (Object value : actualValues) {
          if (matchesExpected(value, wanted, nested)) { found = true; break; }
        }
        if (!found) return false;
      }
      return true;
    }
    return Eq.eq(actual, expected);
  }

  private static boolean matchesExpected(Object actual, Object expected, boolean nested) {
    if (expected instanceof HaraMatcher matcher) return matcher.matches(actual);
    if (expected instanceof HaraFunction || expected instanceof Class<?>) return predicateMatches(expected, actual);
    if (nested && (expected instanceof IMapType<?, ?> || expected instanceof ILinearType<?>)) {
      return containsMatches(actual, expected, true, false, false, false);
    }
    return Eq.eq(actual, expected);
  }

  @Override
  public String toString() {
    return kind == Kind.ANYTHING ? "anything" : kind.name().toLowerCase() + "(" + Objects.toString(expected) + ")";
  }
}
