package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import java.util.Arrays;
import org.junit.Test;

public class HaraCliRouterTest {
  @Test
  public void everyPublicManifestRouteResolvesToItsStableId() {
    HaraCliRouter router = HaraCliRouter.instance();
    for (HaraCliRouter.Route route : router.publicRoutes()) {
      java.util.ArrayList<String> argv = new java.util.ArrayList<>(route.path());
      if ("hara.cli.route/run-file".equals(route.id())) argv.add("fixture.hal");
      HaraCliRouter.Resolution resolution =
          router.resolve(argv.toArray(new String[0]));
      assertEquals(route.id(), resolution.route().id());
    }
  }

  @Test
  public void canonicalAndAliasRoutesHaveTheSameStableId() {
    HaraCliRouter router = HaraCliRouter.instance();
    HaraCliRouter.Resolution canonical =
        router.resolve(new String[] {"project", "check", "demo"});
    HaraCliRouter.Resolution alias =
        router.resolve(new String[] {"check", "demo"});

    assertEquals("hara.cli.route/project-check", canonical.route().id());
    assertEquals(canonical.route().id(), alias.route().id());
    assertEquals(java.util.List.of("demo"), canonical.arguments());
    assertFalse(canonical.alias());
    assertTrue(alias.alias());
  }

  @Test
  public void nestedOperationIdentityIsPreservedForTheLegacyAdapter() {
    assertEquals(
        Arrays.asList("spec", "check-contribution", "candidate"),
        Arrays.asList(
            HaraCliRouter.instance()
                .normalize(new String[] {"spec", "check-contribution", "candidate"})));
  }

  @Test
  public void runFileAndProjectRunAreUnambiguous() {
    HaraCliRouter router = HaraCliRouter.instance();
    assertEquals(
        "hara.cli.route/project-run",
        router.resolve(new String[] {"run"}).route().id());
    assertEquals(
        "hara.cli.route/run-file",
        router.resolve(new String[] {"run", "main.hal"}).route().id());
  }

  @Test
  public void groupedProjectRoutesNormalizeToClosedLegacyHandlers() {
    assertTrue(
        Arrays.equals(
            new String[] {"check", "demo"},
            HaraCliRouter.instance()
                .normalize(new String[] {"project", "check", "demo"})));
  }

  @Test
  public void platformManagementRoutesNormalizeToClosedHandlers() {
    assertTrue(
        Arrays.equals(
            new String[] {"id", "enroll", "--owner", "alice"},
            HaraCliRouter.instance()
                .normalize(new String[] {"id", "enroll", "--owner", "alice"})));
    assertTrue(
        Arrays.equals(
            new String[] {"asset", "build", "demo"},
            HaraCliRouter.instance()
                .normalize(new String[] {"asset", "build", "demo"})));
    assertTrue(
        Arrays.equals(
            new String[] {"tap", "verify", "hara"},
            HaraCliRouter.instance()
                .normalize(new String[] {"tap", "verify", "hara"})));
    assertTrue(
        Arrays.equals(
            new String[] {"extension", "check", "demo"},
            HaraCliRouter.instance()
                .normalize(new String[] {"package", "extension", "check", "demo"})));
  }

  @Test
  public void developerCommandsAreNotPublishedAsPublicRoutes() {
    assertTrue(
        HaraCliRouter.instance().publicRoutes().stream()
            .noneMatch(route -> route.path().equals(java.util.List.of("benchmark"))));
  }
}
