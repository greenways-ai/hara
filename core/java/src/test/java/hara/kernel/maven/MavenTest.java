package hara.kernel.maven;

import hara.kernel.Foundation;
import hara.kernel.protocol.IRuntime;
import org.junit.Test;

import static org.junit.Assert.assertNotNull;

public class MavenTest {

  @Test
  public void testLoad() {
    Foundation f = new Foundation();
    f.call("SESSION", "NEW", "test");
    IRuntime rt = (IRuntime) f.RTS.get("test");
    assertNotNull(rt);

    f.call("MAVEN", "LOAD", "test", "org.apache.commons:commons-lang3:3.12.0");

    Class<?> stringUtilsClass = rt.classFor("org.apache.commons.lang3.StringUtils");
    assertNotNull(stringUtilsClass);
  }
}
