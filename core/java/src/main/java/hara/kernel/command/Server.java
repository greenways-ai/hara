package hara.kernel.command;

import hara.kernel.Command;
import hara.kernel.Foundation;

import java.util.List;

@Command.Fn(name = "SERVER")
public class Server {

  @Command.Sub(name = "INFO")
  public static Object info(Foundation F, List<Object> args) {
    return null;
  }

  @Command.Sub(name = "LIST")
  public static Object list(Foundation F, List<Object> args) {
    return null;
  }

  @Command.Sub(name = "NEW")
  public static Object newServer(Foundation F, List<Object> args) {
    return null;
  }

  @Command.Sub(name = "STOP")
  public static Object stop(Foundation F, List<Object> args) {
    return null;
  }

  @Command.Sub(name = "EXISTS")
  public static Object exists(Foundation F, List<Object> args) {
    return null;
  }
}
