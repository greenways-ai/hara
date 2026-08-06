package hara.kernel.command;

import hara.kernel.Command;
import hara.kernel.Foundation;
import hara.lang.base.Ex;

import java.util.List;

@Command.Fn(name = "OS")
public class Os {

  @Command.Sub(name = "LS")
  public static Object ls(Foundation F, List<Object> args) {
    List<String> sArgs = Foundation.toStringList(args);
    sArgs.add(0, "ls");
    return runProcess(sArgs);
  }

  @Command.Sub(name = "PWD")
  public static Object pwd(Foundation F, List<Object> args) {
    return System.getenv("PWD");
  }

  @Command.Sub(name = "RUN")
  public static Object run(Foundation F, List<Object> args) {
    List<String> sArgs = Foundation.toStringList(args);
    return runProcess(sArgs);
  }

  public static String runProcess(java.util.List<String> args) {
    try {
      var p = new ProcessBuilder().command(args).start();
      return new String(p.getInputStream().readAllBytes());
    } catch (Throwable t) {
      throw Ex.Sneaky(t);
    }
  }
}
