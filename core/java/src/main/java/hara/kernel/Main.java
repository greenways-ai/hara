package hara.kernel;

import hara.kernel.base.RT;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.util.Arrays;

public class Main {

  public static void main(String[] args) throws FileNotFoundException, IOException {
    if (args.length > 0) {
      String command = args[0];
      String[] restArgs = Arrays.copyOfRange(args, 1, args.length);

      switch (command) {
        case "native":
          NativeMain.main(restArgs);
          return;
        case "new":
          Project.create(restArgs.length > 0 ? restArgs[0] : null);
          return;
        case "run":
          Project.run();
          return;
        case "repl":
          Project.repl();
          return;
        case "setup":
          Project.setup(restArgs);
          return;
        default:
          System.out.println("Unknown command: " + command);
          System.out.println("Available commands: native, new, run, repl, setup");
          return;
      }
    }

    // Default behavior: Start standalone REPL (as before)
    var F = new Foundation();
    var server = new Server(F, "PRIMARY", Foundation.DEFAULT_PORT, null);
    var rt = new RT.Instance<>(F, "ROOT");

    F.SERVERS.put(server._key, server);
    F.RTS.put(rt._key, rt);

    server.start();

    // Start REPL
    Repl repl = new Repl(rt);
    repl.run();
  }
}
