package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.fail;

import java.nio.file.Files;
import java.nio.file.Path;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.PolyglotException;
import org.junit.Test;

public class HaraSubstrateProtocolTest {
  @Test
  public void substrateCapabilitiesLoadAndDispatchThroughExtendedTypes() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          42,
          context
              .eval(
                  HaraLanguage.ID,
                  "(require 'std.substrate.protocol) "
                      + "(defstruct Fixture [id]) "
                      + "(extend-type Fixture std.substrate.protocol/IFrame "
                      + "  (frame-id [frame] (field frame :id)) "
                      + "  (frame-kind [frame] :request) "
                      + "  (frame-space [frame] \"space/test\") "
                      + "  (frame-meta [frame] {}) "
                      + "  (frame-cause [frame] nil) "
                      + "  (frame-data [frame] 42)) "
                      + "(protocol-call std.substrate.protocol/IFrame frame-data (Fixture \"frame-1\"))")
              .asLong());
    }
  }

  @Test
  public void missingSubstrateCapabilityImplementationReportsProtocolError() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      PolyglotException error =
          org.junit.Assert.assertThrows(
              PolyglotException.class,
              () ->
                  context.eval(
                      HaraLanguage.ID,
                      "(require 'std.substrate.protocol) "
                          + "(defstruct Incomplete []) "
                          + "(protocol-call std.substrate.protocol/IService get-service "
                          + "(Incomplete) \"cache\")"));
      assertTrue(error.getMessage().contains("IService/get-service"));
    }
  }

  @Test
  public void protocolSurfaceHalFixturePasses() throws Exception {
    String source = Files.readString(Path.of("lib/test/std/substrate/protocol_test.hal"));
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      String result = context.eval(HaraLanguage.ID, source).asString();
      assertTrue(result, !result.contains(":pass false"));
    }
  }

  @Test
  public void atomBackedSubstrateRunsWithoutStudioOrBrowserState() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          42,
          context
              .eval(
                  HaraLanguage.ID,
                  "(require 'std.substrate) "
                      + "(def node (std.substrate/node-create \"node-1\")) "
                      + "(protocol-call std.substrate.protocol/IService set-service node \"cache\" 42) "
                      + "(protocol-call std.substrate.protocol/IService get-service node \"cache\")")
              .asLong());
    }
  }

  @Test
  public void sharedProtocolConformanceFixtureRuns() throws Exception {
    String source = Files.readString(Path.of("lib/test-fixtures/std/substrate/protocol_conformance.hal"));
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals("[40 42]", context.eval(HaraLanguage.ID, source).toString());
    }
  }

  @Test
  public void sharedSubstrateFrameConformanceFixtureRuns() throws Exception {
    String source = Files.readString(Path.of("lib/test-fixtures/std/substrate/frame_conformance.hal"));
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "{\"version\":\"substrate.v1\",\"kind\":\"request\",\"id\":\"req-1\",\"source\":\"client/a\",\"target\":\"server/b\",\"space\":\"workspace/main\",\"meta\":{\"trace\":\"trace-1\"},\"action\":\"math/add\",\"args\":[19,23],\"reply_to\":null,\"status\":null,\"data\":null,\"error\":null,\"signal\":null,\"cause\":null}",
          context.eval(HaraLanguage.ID, source).toString());
      try {
        context.eval(
            HaraLanguage.ID,
            "(do (require 'std.substrate.frame) "
                + "(std.substrate.frame/normalize-frame {:kind :unknown :id \"evt-1\"}))");
        fail("expected invalid substrate frames to throw");
      } catch (PolyglotException expected) {
        // The Hara thrown value is intentionally opaque to the host.
      }
      try {
        context.eval(
            HaraLanguage.ID,
            "(do (require 'std.substrate.frame) "
                + "(std.substrate.frame/decode-frame-json \"{bad\"))");
        fail("expected malformed substrate JSON to throw");
      } catch (PolyglotException expected) {
        // The strict JSON reader reports malformed wire input through Hara.
      }
    }
  }

  @Test
  public void sharedSubstrateNodeLifecycleFixtureRuns() throws Exception {
    String source = Files.readString(Path.of("lib/test-fixtures/std/substrate/node_lifecycle_conformance.hal"));
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals("[84 42 :rejected]", context.eval(HaraLanguage.ID, source).toString());
    }
  }

  @Test
  public void substrateNodeHalFixturePasses() throws Exception {
    String source = Files.readString(Path.of("lib/test/std/substrate/node_test.hal"));
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      String result = context.eval(HaraLanguage.ID, source).asString();
      assertTrue(result, !result.contains(":pass false"));
    }
  }

  @Test
  public void substrateRoutesStreamsAndSettlesTransportRequests() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          84,
          context
              .eval(
                  HaraLanguage.ID,
                  "(require 'std.substrate) "
                      + "(def node (std.substrate/node-create \"node-1\")) "
                      + "(protocol-call std.substrate.protocol/ITransport attach-transport node \"peer-a\" "
                      + "  (fn [frame] "
                      + "    (protocol-call std.substrate.protocol/ITransport receive-frame node "
                      + "      (std.substrate/node-frame :response \"res-1\" \"main\" {} nil [] "
                      + "        (protocol-call std.substrate.protocol/IFrame frame-id frame) :ok 84 nil nil nil) "
                      + "      {:transport-id \"peer-a\"}))) "
                      + "(def reply (protocol-call std.substrate.protocol/IRequest request node \"main\" \"sum\" [] "
                      + "  {:id \"req-1\" :transport-id \"peer-a\"})) "
                      + "(promise/value reply)")
              .asLong());
    }
  }

  @Test
  public void substrateCancellationSettlesThePendingPromise() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          ":rejected",
          context
              .eval(
                  HaraLanguage.ID,
                  "(require 'std.substrate) "
                      + "(def node (std.substrate/node-create \"node-1\")) "
                      + "(protocol-call std.substrate.protocol/ITransport attach-transport node \"peer-a\" (fn [frame] nil)) "
                      + "(def pending (protocol-call std.substrate.protocol/IRequest request node \"main\" \"wait\" [] "
                      + "  {:id \"req-cancel\" :transport-id \"peer-a\"})) "
                      + "(protocol-call std.substrate.protocol/IRequest cancel-request node \"req-cancel\" :cancelled) "
                      + "(promise/state pending)")
              .toString());
    }
  }

  @Test
  public void missingLocalRequestHandlerFailsLikeXTalk() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      org.junit.Assert.assertThrows(
          PolyglotException.class,
          () ->
              context.eval(
                  HaraLanguage.ID,
                  "(require 'std.substrate) "
                      + "(def node (std.substrate/node-create \"node-1\")) "
                      + "(protocol-call std.substrate.protocol/IRequest request node \"main\" \"missing\" [] {})"));
    }
  }
}
