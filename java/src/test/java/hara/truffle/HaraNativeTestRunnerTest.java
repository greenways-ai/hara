package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import java.nio.file.Path;
import org.junit.Test;

public final class HaraNativeTestRunnerTest {
  private static final Path ROOT = Path.of(".").toAbsolutePath().normalize();

  @Test
  public void classifiesPassingCodeTestSummary() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(
            ROOT, ROOT.resolve("lib/test-fixtures/std/native/test_runner_pass.hal"));

    assertTrue(result.passed());
    assertEquals(1, result.facts());
    assertEquals(1, result.checks());
    assertEquals(1, result.passedChecks());
    assertEquals(0, result.failedChecks());
  }

  @Test
  public void preservesFailingSummaryForHostReporting() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(
            ROOT, ROOT.resolve("lib/test-fixtures/std/native/test_runner_fail.hal"));

    assertFalse(result.passed());
    assertEquals(1, result.facts());
    assertEquals(1, result.checks());
    assertEquals(0, result.passedChecks());
    assertEquals(1, result.failedChecks());
    assertTrue(result.failureMessage().contains(":failed"));
  }

  @Test
  public void runsPortableFoundationXtalkCommonMathSuite() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(
            ROOT, ROOT.resolve("lib/test/xt/lang/common_math_test.hal"));

    assertTrue(result.failureMessage(), result.passed());
    assertEquals(29, result.facts());
    assertEquals(87, result.checks());
    assertEquals(87, result.passedChecks());
    assertEquals(0, result.failedChecks());
    assertEquals(0, result.errors());
    assertEquals(0, result.timeouts());
  }

  @Test
  public void runsPortableFoundationXtalkCommonLibSuite() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(
            ROOT, ROOT.resolve("lib/test/xt/lang/common_lib_test.hal"));

    assertTrue(result.failureMessage(), result.passed());
    assertEquals(39, result.facts());
    assertEquals(116, result.checks());
    assertEquals(116, result.passedChecks());
    assertEquals(0, result.failedChecks());
    assertEquals(0, result.errors());
    assertEquals(0, result.timeouts());
  }

  @Test
  public void runsPortableFoundationXtalkFoundationLayer() throws Exception {
    String[] files =
        System.getProperty(
                "hara.xt.test",
                "common_color_test.hal,common_data_test.hal,common_iter_test.hal,"
                    + "common_lib_test.hal,common_math_test.hal,common_module_test.hal,"
                    + "common_notify_test.hal,common_promise_test.hal,"
                    + "common_protocol_test.hal,common_repl_test.hal,"
                    + "common_resource_test.hal,common_sort_by_test.hal,"
                    + "common_sort_topo_test.hal,common_string_test.hal,"
                    + "common_trace_test.hal,common_tree_test.hal,parser_xml_test.hal,"
                    + "spec_base_test.hal,spec_bytes_test.hal,spec_link_test.hal,"
                    + "spec_os_test.hal,spec_primitive_test.hal,spec_promise_test.hal")
            .split(",");
    for (String file : files) {
      HaraNativeTestRunner.Result result =
          HaraNativeTestRunner.runFile(ROOT, ROOT.resolve("lib/test/xt/lang/" + file));
      assertTrue(result.failureMessage(), result.passed());
      assertEquals(0, result.failedChecks());
      assertEquals(0, result.errors());
      assertEquals(0, result.timeouts());
    }
  }

  @Test
  public void runsPortablePostgresCoreCompilerSlice() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(ROOT, ROOT.resolve("lib/test/postgres/core_test.hal"));

    assertTrue(result.failureMessage(), result.passed());
    assertEquals(12, result.facts());
    assertEquals(28, result.checks());
    assertEquals(28, result.passedChecks());
    assertEquals(0, result.failedChecks());
    assertEquals(0, result.errors());
    assertEquals(0, result.timeouts());
  }

  @Test
  public void runsPortablePostgresCoreImplBase() throws Exception {
    String[][] suites = {
      {"lib/test/postgres/core/impl_base_test.hal", "15", "31"},
      {"lib/test/postgres/core/impl_main_test.hal", "8", "14"},
      {"lib/test/postgres/core/impl_insert_test.hal", "7", "8"},
      {"lib/test/postgres/core/impl_update_test.hal", "7", "8"},
      {"lib/test/postgres/core/impl_test.hal", "4", "8"},
      {"lib/test/postgres/core/graph_walk_test.hal", "4", "6"}
    };
    for (String[] suite : suites) {
      HaraNativeTestRunner.Result result =
          HaraNativeTestRunner.runFile(ROOT, ROOT.resolve(suite[0]));
      int facts = Integer.parseInt(suite[1]);
      int checks = Integer.parseInt(suite[2]);
      assertTrue(result.failureMessage(), result.passed());
      assertEquals(facts, result.facts());
      assertEquals(checks, result.checks());
      assertEquals(checks, result.passedChecks());
      assertEquals(0, result.failedChecks());
      assertEquals(0, result.errors());
      assertEquals(0, result.timeouts());
    }
  }

  @Test
  public void runsPortablePostgresTypedFoundation() throws Exception {
    String[][] suites = {
      {"lib/test/postgres/typed_test.hal", "8", "9"},
      {"lib/test/postgres/typed/typed_common_test.hal", "11", "37"},
      {"lib/test/postgres/typed/typed_shape_test.hal", "7", "11"},
      {"lib/test/postgres/typed/typed_resolve_test.hal", "6", "16"},
      {"lib/test/postgres/typed/typed_jsonb_test.hal", "12", "30"},
      {"lib/test/postgres/typed/typed_infer_test.hal", "10", "15"},
      {"lib/test/postgres/typed/typed_parse_test.hal", "11", "24"},
      {"lib/test/postgres/typed/typed_analyze_test.hal", "27", "18"},
      {"lib/test/postgres/typed/export/json_openapi_test.hal", "5", "12"},
      {"lib/test/postgres/typed/export/json_schema_test.hal", "5", "9"},
      {"lib/test/postgres/typed/export/server_api_test.hal", "5", "6"},
      {"lib/test/postgres/typed/export/server_db_test.hal", "8", "16"},
      {"lib/test/postgres/typed/export/ts_schema_test.hal", "4", "11"}
    };

    for (String[] suite : suites) {
      HaraNativeTestRunner.Result result =
          HaraNativeTestRunner.runFile(ROOT, ROOT.resolve(suite[0]));
      int facts = Integer.parseInt(suite[1]);
      int checks = Integer.parseInt(suite[2]);

      assertTrue(result.failureMessage(), result.passed());
      assertEquals(facts, result.facts());
      assertEquals(checks, result.checks());
      assertEquals(checks, result.passedChecks());
      assertEquals(0, result.failedChecks());
      assertEquals(0, result.errors());
      assertEquals(0, result.timeouts());
    }
  }
}
