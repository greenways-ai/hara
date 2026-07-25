package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.Value;
import org.graalvm.polyglot.io.IOAccess;
import org.junit.Test;

/** Executes the guest-language programs used by the namespace walkthroughs. */
public class NamespaceWalkthroughTest {
  @Test
  public void loadsAndCallsTheServiceProjectByNamespace() {
    Path root = Path.of("lib", "examples", "services").toAbsolutePath().normalize();
    try (Context context =
        Context.newBuilder(HaraLanguage.ID)
            .currentWorkingDirectory(root)
            .allowIO(IOAccess.ALL)
            .build()) {
      context.eval(HaraLanguage.ID, "(require 'services.api)");
      Value result =
          context.eval(
              HaraLanguage.ID,
              "(let [response (services.api/dispatch "
                  + "\"  /STATUS  \" {:id 42 :action :health-check})] "
                  + "[(:route response) "
                  + " (:worker (:result response)) "
                  + " (:status (:result response)) "
                  + " (:id (:job (:result response)))])");

      assertEquals("/status", result.getArrayElement(0).asString());
      assertEquals("background", result.getArrayElement(1).asString());
      assertEquals(":processed", result.getArrayElement(2).toString());
      assertEquals(42, result.getArrayElement(3).asInt());
    }
  }

  @Test
  public void runsEveryServiceNamespaceFactInHara() {
    Path root = Path.of("lib", "examples", "services").toAbsolutePath().normalize();
    try (Context context =
        Context.newBuilder(HaraLanguage.ID)
            .currentWorkingDirectory(root)
            .allowIO(IOAccess.ALL)
            .build()) {
      context.eval(HaraLanguage.ID, "(require 'services.api-test)");
      Value results =
          context.eval(
              HaraLanguage.ID,
              "(code.test/run {:namespace \"services.api-test\"})");

      assertEquals(3, results.getArraySize());
      assertAllPass(results);
    }
  }

  @Test
  public void publishedCodeTestExamplesRemainExecutable() throws Exception {
    assertExamplePasses("basic.hal", 3);
    assertExamplePasses("advanced.hal", 4);

    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      Value task =
          context.eval(
              HaraLanguage.ID,
              Files.readString(
                  Path.of("lib", "examples", "code-test", "tasks.hal"), StandardCharsets.UTF_8));
      assertEquals(3, task.getArraySize());
      assertEquals(2, task.getArrayElement(0).asInt());
      assertEquals(6, task.getArrayElement(2).asInt());
    }
  }

  private static void assertExamplePasses(String file, int expectedFacts) throws Exception {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      Value results =
          context.eval(
              HaraLanguage.ID,
              Files.readString(Path.of("lib", "examples", "code-test", file), StandardCharsets.UTF_8));
      assertEquals(expectedFacts, results.getArraySize());
      assertAllPass(results);
    }
  }

  private static void assertAllPass(Value results) {
    for (long i = 0; i < results.getArraySize(); i++) {
      Value result = results.getArrayElement(i);
      assertTrue(result.toString(), "PASS".equals(result.getHashValue("status").asString()));
    }
  }
}
