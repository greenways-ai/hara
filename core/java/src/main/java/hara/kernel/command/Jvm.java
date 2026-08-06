package hara.kernel.command;

import hara.kernel.Command;
import hara.kernel.Foundation;

import java.util.List;

@Command.Fn(name = "JVM")
public class Jvm {

  @Command.Sub(name = "BOOTPATH")
  public static Object bootpath(Foundation F, List<Object> args) {
    return System.getProperty("sun.boot.library.path");
  }

  @Command.Sub(name = "CP")
  public static Object cp(Foundation F, List<Object> args) {
    return System.getProperty("java.class.path");
  }

  @Command.Sub(name = "CLASSPATH")
  public static Object classpath(Foundation F, List<Object> args) {
    return System.getProperty("java.class.path");
  }

  @Command.Sub(name = "CLASSLOADER")
  public static Object classloader(Foundation F, List<Object> args) {
    return ClassLoader.getSystemClassLoader().toString();
  }

  @Command.Sub(name = "ENV")
  public static Object env(Foundation F, List<Object> args) {
    List<String> sArgs = Foundation.toStringList(args);
    return (sArgs.size() == 0)
        ? Foundation.mapToList(System.getenv())
        : System.getenv(sArgs.get(0));
  }

  @Command.Sub(name = "HOME")
  public static Object home(Foundation F, List<Object> args) {
    return System.getProperty("java.home");
  }

  @Command.Sub(name = "PROPS")
  public static Object props(Foundation F, List<Object> args) {
    List<String> sArgs = Foundation.toStringList(args);
    return (sArgs.size() == 0)
        ? Foundation.mapToList(System.getProperties())
        : System.getProperty(sArgs.get(0));
  }

  @Command.Sub(name = "VENDOR")
  public static Object vendor(Foundation F, List<Object> args) {
    return System.getProperty("java.vendor");
  }

  @Command.Sub(name = "VERSION")
  public static Object version(Foundation F, List<Object> args) {
    return System.getProperty("java.version");
  }
}
