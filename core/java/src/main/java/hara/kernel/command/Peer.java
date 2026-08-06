package hara.kernel.command;

import hara.kernel.Command;
import hara.kernel.Foundation;

import java.util.List;

@Command.Fn(name = "PEER")
public class Peer {

  @Command.Sub(name = "LIST")
  public static Object list(Foundation F, List<Object> args) {
    return Foundation.mapToList(F.PEERS);
  }

  @Command.Sub(name = "ADD")
  public static Object add(Foundation F, List<Object> args) {
    List<String> sArgs = Foundation.toStringList(args);
    // PEER ADD name host port
    String name = sArgs.get(0);
    String host = sArgs.get(1);
    int port = Integer.parseInt(sArgs.get(2));
    F.PEERS.put(name, new Foundation.Peer(name, host, port));
    return name;
  }

  @Command.Sub(name = "REMOVE")
  public static Object remove(Foundation F, List<Object> args) {
    List<String> sArgs = Foundation.toStringList(args);
    return F.PEERS.remove(sArgs.get(0)) != null;
  }

  @Command.Sub(name = "PING")
  public static Object ping(Foundation F, List<Object> args) {
    List<String> sArgs = Foundation.toStringList(args);
    // Minimal implementation: check if we have a record
    return F.PEERS.containsKey(sArgs.get(0));
  }
}
