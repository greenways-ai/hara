package hara.kernel;

import hara.kernel.protocol.IRuntime;
import hara.lang.base.Ex;
import hara.lang.base.Iter;
import hara.lang.base.primitive.Array;
import hara.lang.protocol.IContext;
import hara.lang.protocol.IDisplay;
import hara.lang.protocol.INamespaced;

import java.lang.reflect.Method;
import java.util.Arrays;
import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.stream.Collectors;

@SuppressWarnings("rawtypes")
public class Foundation implements IContext {

  public static final int DEFAULT_PORT = 1311;

  public static class Peer {
    public final String name;
    public final String host;
    public final int port;

    public Peer(String name, String host, int port) {
      this.name = name;
      this.host = host;
      this.port = port;
    }

    @Override
    public String toString() {
      return name + "@" + host + ":" + port;
    }
  }

  public final ConcurrentHashMap<String, Server> SERVERS = new ConcurrentHashMap<String, Server>();
  public final ConcurrentHashMap<String, IRuntime> RTS = new ConcurrentHashMap<String, IRuntime>();
  public final ConcurrentHashMap<String, Peer> PEERS = new ConcurrentHashMap<String, Peer>();
  public final ConcurrentHashMap<String, Command.Type> REGISTRY = new ConcurrentHashMap<>();

  public Foundation() {
    loadCommands(NativeMode.commandClasses());
  }

  public void register(String name, Command.Type cmd) {
    REGISTRY.put(name, cmd);
  }

  public void loadCommands(java.util.List<Class<?>> classes) {
    for (Class<?> cls : classes) {
      // Check for Class-level Command.Fn
      Command.Fn classCmd = cls.getAnnotation(Command.Fn.class);
      if (classCmd != null) {
        // Build Dispatcher
        Map<String, Method> subCommands = new HashMap<>();
        for (Method method : cls.getDeclaredMethods()) {
          Command.Sub sub = method.getAnnotation(Command.Sub.class);
          if (sub != null) {
            subCommands.put(sub.name(), method);
          }
        }

        register(
            classCmd.name(),
            (F, args) -> {
              if (args.isEmpty()) {
                // Default to HELP if no subcommand
                java.util.Set<String> keys = new java.util.HashSet<>(subCommands.keySet());
                keys.add("HELP");
                return Iter.toArrayList(Array.toIter(keys.toArray()));
              }

              String subName = args.get(0).toString().toUpperCase();

              // Auto-HELP
              if ("HELP".equals(subName)) {
                java.util.Set<String> keys = new java.util.HashSet<>(subCommands.keySet());
                keys.add("HELP");
                return Iter.toArrayList(Array.toIter(keys.toArray()));
              }

              Method m = subCommands.get(subName);
              if (m != null) {
                args.remove(0); // Consume subcommand
                try {
                  return m.invoke(null, F, args);
                } catch (Exception e) {
                  throw Ex.Sneaky(e);
                }
              }
              throw new Ex.Unsupported("Unknown subcommand: " + subName);
            });
      } else {
        // Scan for Method-level Command.Fn
        for (Method method : cls.getDeclaredMethods()) {
          Command.Fn annotation = method.getAnnotation(Command.Fn.class);
          if (annotation != null) {
            register(
                annotation.name(),
                (F, args) -> {
                  try {
                    return method.invoke(null, F, args);
                  } catch (Exception e) {
                    throw Ex.Sneaky(e);
                  }
                });
          }
        }
      }
    }
  }

  @SuppressWarnings("unchecked")
  public static java.util.List<String> toStringList(java.util.List<Object> args) {
    return args.stream().map(Object::toString).collect(Collectors.toList());
  }

  @SuppressWarnings("unchecked")
  public static Object mapToList(Object m) {
    if (m instanceof Map) {
      return ((Map) m)
          .entrySet().stream()
              .map(
                  e ->
                      Arrays.asList(
                          ((Map.Entry) e).getKey(), mapToList(((Map.Entry) e).getValue())))
              .collect(Collectors.toList());
    } else {
      return m;
    }
  }

  @SuppressWarnings("unchecked")
  public static Object runCommand(Foundation F, java.util.List<Object> args) {
    if (args.isEmpty()) {
      return Iter.toArrayList(Array.toIter(F.REGISTRY.keySet().toArray()));
    }

    Object cmdObj = args.get(0);
    String name;

    if (cmdObj instanceof INamespaced) {
      name = ((INamespaced) cmdObj).getName();
    } else if (cmdObj instanceof IDisplay) {
      String display = ((IDisplay) cmdObj).display();
      name = display.startsWith(":") ? display.substring(1) : display;
    } else {
      name = cmdObj.toString();
    }

    name = name.toUpperCase();

    args.remove(0);

    Command.Type cmd = F.REGISTRY.get(name);
    if (cmd != null) {
      return cmd.apply(F, args);
    }

    throw new Ex.Unsupported("Unknown command: " + name);
  }

  @SuppressWarnings("unchecked")
  @Override
  public Object call(Object... args) {
    java.util.List<Object> inputs = Iter.toArrayList(Iter.iter(args));
    return runCommand(this, inputs);
  }
}
