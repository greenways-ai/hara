package hara.kernel.command;

import hara.kernel.Command;
import hara.kernel.Foundation;
import hara.kernel.NativeMode;
import hara.kernel.base.RT;
import hara.lang.base.Ex;

import java.util.List;

@Command.Fn(name = "MAVEN")
public class Maven {

  @Command.Sub(name = "LOAD")
  public static Object load(Foundation F, List<Object> args) {
    NativeMode.requireDisabled("dynamic Maven dependency loading");
    List<String> sArgs = Foundation.toStringList(args);
    String key = sArgs.get(0);
    RT.Instance rt = (RT.Instance) F.RTS.get(key);
    if (rt == null) {
      throw new Ex.Runtime("No Session: " + key);
    }
    return hara.kernel.maven.Maven.load.invoke(rt, sArgs.get(1));
  }
}
