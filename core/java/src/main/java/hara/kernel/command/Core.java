package hara.kernel.command;

import hara.compiler.Compiler;
import hara.compiler.DynamicClassLoader;
import hara.kernel.Command;
import hara.kernel.Foundation;
import hara.kernel.NativeMode;
import hara.kernel.base.Parser;
import hara.kernel.base.RT;
import hara.lang.base.Ex;
import hara.lang.base.G;
import hara.lang.base.Iter;
import hara.lang.base.primitive.Array;

import java.util.Arrays;
import java.util.List;
import java.util.Map;

public class Core {

  @Command.Fn(name = "HELP")
  public static Object cmdHELP(Foundation F, List<Object> args) {
    return Iter.toArrayList(
        Iter.map(Array.toIter(F.REGISTRY.keySet().toArray()), (x) -> x.toString()));
  }

  @Command.Fn(name = "SHUTDOWN")
  public static Object cmdSHUTDOWN(Foundation F, List<Object> args) {
    System.exit(1);
    return null;
  }

  @Command.Fn(name = "PING")
  public static Object cmdPING(Foundation F, List<Object> args) {
    return "PONG";
  }

  @Command.Fn(name = "ECHO")
  public static Object cmdECHO(Foundation F, List<Object> args) {
    return args;
  }

  @Command.Fn(name = "DIR")
  public static Object cmdDIR(Foundation F, List<Object> args) {
    return Arrays.asList(
        "SERVERS", Iter.toArrayList(F.SERVERS.keys()),
        "RTS", Iter.toArrayList(F.RTS.keys()),
        "PEERS", Iter.toArrayList(F.PEERS.keys()));
  }

  @Command.Fn(name = "INFO")
  public static Object runInfo(Foundation F, List<Object> args) {
    return Foundation.mapToList(
        Map.of(
            "java.version", System.getProperty("java.version"),
            "os.name", System.getProperty("os.name"),
            "sessions", F.RTS.size(),
            "servers", F.SERVERS.size(),
            "peers", F.PEERS.size()));
  }

  @Command.Fn(name = "EVAL")
  public static Object runEval(Foundation F, List<Object> args) {
    String key = args.get(0).toString();
    RT.Instance rt = (RT.Instance) F.RTS.get(key);
    if (rt == null) {
      throw new Ex.Runtime("No Session: " + key);
    }
    return G.display(rt.eval(rt.readString(args.get(1).toString())));
  }

  @Command.Fn(name = "COMPILE")
  public static Object runCompile(Foundation F, List<Object> args) {
    NativeMode.requireDisabled("runtime compilation");
    try {
      hara.lang.data.List expression =
          (hara.lang.data.List) Parser.LispReader.readString(args.get(0).toString(), null);
      Compiler compiler = new Compiler();
      byte[] bytecode = compiler.compile(expression);
      DynamicClassLoader loader = new DynamicClassLoader(Foundation.class.getClassLoader());
      Class<?> clazz = loader.defineClass(null, bytecode);
      return clazz.getConstructor().newInstance();
    } catch (Exception e) {
      throw Ex.Sneaky(e);
    }
  }
}
