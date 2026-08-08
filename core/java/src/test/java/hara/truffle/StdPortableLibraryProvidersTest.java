package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import java.util.Set;
import java.util.ServiceLoader;
import java.util.stream.Collectors;
import java.util.stream.StreamSupport;
import org.graalvm.polyglot.Context;
import org.junit.Test;

public class StdPortableLibraryProvidersTest {
  @Test
  public void portableNamespacesDoNotRequireProviders() {
    Set<String> namespaces =
        StreamSupport.stream(
                ServiceLoader.load(HaraLibraryProvider.class).spliterator(), false)
            .map(HaraLibraryProvider::namespace)
            .collect(Collectors.toSet());

    assertFalse(namespaces.contains("std.lib.zip"));
    assertFalse(namespaces.contains("std.block"));
    assertFalse(namespaces.contains("std.lib.task"));
    assertFalse(namespaces.contains("std.logic.kanren"));
    assertFalse(namespaces.contains("code.test"));
    assertTrue(namespaces.contains("std.foundation.coroutine"));
  }

  @Test
  public void canonicalPortableLibrariesLoadAndExecuteTogether() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[[1 3] \"[1 3]\" 8 :passed]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(ns portable.provider-probe "
                      + "(:require [std.lib.zip :as zip] "
                      + "[std.block :as block] "
                      + "[std.lib.task :as task] "
                      + "[code.test :as test])) "
                      + "(task/deftask double-task "
                      + "{:template :default :main {:fn (fn [value] (* 2 value))}}) "
                      + "(test/purge-all) "
                      + "(test/register! \"provider fact\" "
                      + "(fn [options] [{:pass true :actual 1 :expected 1}])) "
                      + "[(zip/result "
                      + "(zip/replace-right "
                      + "(zip/step-right (zip/step-inside (zip/vector-zip [1 2]))) 3)) "
                      + "(block/string (block/parse-first \"[1 3]\")) "
                      + "(task/invoke double-task 4) "
                      + "(get (test/run {:namespace \"portable.provider-probe\"}) :status)]")
              .toString());
    }
  }
}
