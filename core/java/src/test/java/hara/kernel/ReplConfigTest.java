package hara.kernel;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import java.nio.file.Path;
import org.junit.Test;

public class ReplConfigTest {
  @Test
  public void rendersPlainBannerAndPrompt() {
    ReplConfig config = new ReplConfig(Path.of("history"), ReplConfig.DEFAULT_SPLASH, false);

    String banner = config.banner("JVM interpreter", "dev");
    assertTrue(banner.contains("░░░▒▒▓▒▒░░░"));
    assertTrue(banner.contains("▓▓▓▓"));
    assertFalse(banner.contains("◉"));
    String[] splashLines = ReplConfig.DEFAULT_SPLASH.stripTrailing().split("\\R");
    assertEquals("", splashLines[0]);
    assertEquals("", splashLines[1]);
    assertEquals(41, splashLines[5].stripLeading().length());
    assertEquals(58, splashLines[10].length());
    assertTrue(banner.contains("JVM interpreter"));
    assertTrue(banner.contains("session dev"));
    assertTrue(banner.contains("/help"));
    assertFalse(banner.contains("\u001b["));
    assertEquals("hara[user] ", config.prompt("user"));
    assertEquals("[user] ", config.sessionPrompt("user"));
    assertEquals("       … ", config.continuationPrompt());
  }

  @Test
  public void acceptsCustomSplashAndColor() {
    ReplConfig config = new ReplConfig(Path.of("history"), "HELLO HARA", true);

    assertTrue(config.banner("Truffle", "polyglot").contains("HELLO HARA"));
    assertTrue(config.banner("Truffle", "polyglot").contains("\u001b["));
    assertEquals("quiet", config.withSplash("quiet").splash());
    String tagline = config.tagline("JOURNEY WITHIN");
    assertTrue(tagline.contains("\u001b[38;2;"));
    assertFalse(tagline.contains("\u001b[48;"));
    assertEquals("JOURNEY WITHIN", config.withColor(false).tagline("JOURNEY WITHIN"));
  }
}
