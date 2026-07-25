package hara.truffle;

import hara.lang.data.Keyword;
import hara.lang.data.types.IMapType;
import java.io.IOException;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import std.lib.resp.RespConnection;

/** Blocking RESP2 client exported as {@code std.resp.client/*}. */
public final class StdRespClient {
  private StdRespClient() {}

  @HaraExport(
      name = "connect",
      doc = "Connects to a RESP endpoint. Network capability is required.",
      arglists = {"[host port]", "[host port options]"})
  public static Object connect(HaraContext context, Object[] values) {
    if (values.length < 2 || values.length > 3) {
      throw failure("connect", "expects a host, port, and optional options map");
    }
    context.requireSocketIO("std.resp.client/connect");
    String host = String.valueOf(HaraBox.unwrap(values[0]));
    int port = integer(values[1], "connect", 0, 65535);
    Object options = values.length == 3 ? HaraBox.unwrap(values[2]) : null;
    int connectTimeout = optionInteger(options, "connect-timeout-ms", 5000, 0, Integer.MAX_VALUE);
    int readTimeout = optionInteger(options, "read-timeout-ms", 0, 0, Integer.MAX_VALUE);
    boolean bulkBytes = "bytes".equals(optionName(options, "decode-bulk", "string"));
    try {
      Socket socket = new Socket();
      socket.connect(new InetSocketAddress(host, port), connectTimeout);
      socket.setSoTimeout(readTimeout);
      return new Client(new RespConnection(socket), bulkBytes);
    } catch (IOException error) {
      throw failure("connect", error.getMessage());
    }
  }

  @HaraExport(
      name = "call",
      doc = "Writes one command and blocks for its response.",
      arglists = {"[client command & args]"})
  public static Object call(HaraContext context, Object[] values) {
    if (values.length < 2) throw failure("call", "expects a client and command");
    Client client = client(values[0], "call");
    ArrayList<Object> command = new ArrayList<>();
    for (int i = 1; i < values.length; i++) command.add(toWire(values[i]));
    synchronized (client) {
      ensureOpen(client, "call");
      try {
        client.connection.write(command);
        return fromWire(client, client.connection.read());
      } catch (IOException error) {
        throw failure("call", error.getMessage());
      }
    }
  }

  @HaraExport(
      name = "write",
      doc = "Writes and flushes one raw RESP value.",
      arglists = {"[client value]"})
  public static Object write(HaraContext context, Object[] values) {
    requireArity(values, 2, "write");
    Client client = client(values[0], "write");
    synchronized (client) {
      ensureOpen(client, "write");
      try {
        client.connection.writeValue(toWire(values[1]));
        return client;
      } catch (IOException error) {
        throw failure("write", error.getMessage());
      }
    }
  }

  @HaraExport(
      name = "read",
      doc = "Blocks until one raw RESP value is available.",
      arglists = {"[client]"})
  public static Object read(HaraContext context, Object[] values) {
    requireArity(values, 1, "read");
    Client client = client(values[0], "read");
    synchronized (client) {
      ensureOpen(client, "read");
      try {
        return fromWire(client, client.connection.read());
      } catch (IOException error) {
        throw failure("read", error.getMessage());
      }
    }
  }

  @HaraExport(
      name = "pipeline",
      doc = "Writes command vectors and returns their responses in order.",
      arglists = {"[client commands]"})
  public static Object pipeline(HaraContext context, Object[] values) {
    requireArity(values, 2, "pipeline");
    Client client = client(values[0], "pipeline");
    Object commands = HaraBox.unwrap(values[1]);
    if (!(commands instanceof Iterable<?> iterable)) {
      throw failure("pipeline", "expects a sequential collection of commands");
    }
    synchronized (client) {
      ensureOpen(client, "pipeline");
      int count = 0;
      try {
        for (Object command : iterable) {
          Object wire = toWire(command);
          if (!(wire instanceof List<?> list)) {
            throw failure("pipeline", "expects every command to be sequential");
          }
          client.connection.write(list);
          count++;
        }
        ArrayList<Object> responses = new ArrayList<>(count);
        for (int i = 0; i < count; i++) {
          responses.add(fromWire(client, client.connection.read()));
        }
        return responses;
      } catch (IOException error) {
        throw failure("pipeline", error.getMessage());
      }
    }
  }

  @HaraExport(
      name = "open?",
      doc = "Returns true while the client socket remains open.",
      arglists = {"[client]"})
  public static Object open(HaraContext context, Object[] values) {
    requireArity(values, 1, "open?");
    Client client = client(values[0], "open?");
    return !client.closed && !client.connection.socket.isClosed();
  }

  @HaraExport(
      name = "close",
      doc = "Closes a RESP client. Repeated closes are safe.",
      arglists = {"[client]"})
  public static Object close(HaraContext context, Object[] values) {
    requireArity(values, 1, "close");
    Client client = client(values[0], "close");
    synchronized (client) {
      if (!client.closed) {
        try {
          client.connection.close();
        } catch (IOException error) {
          throw failure("close", error.getMessage());
        } finally {
          client.closed = true;
        }
      }
      return true;
    }
  }

  private static Object fromWire(Client client, Object value) {
    if (value instanceof RespConnection.Parser.ServerError error) {
      throw failure("server-error", error.getMessage());
    }
    if (value instanceof byte[] bytes) {
      return client.bulkBytes ? bytes : new String(bytes, StandardCharsets.UTF_8);
    }
    if (value instanceof List<?> list) {
      ArrayList<Object> result = new ArrayList<>(list.size());
      for (Object item : list) result.add(fromWire(client, item));
      return result;
    }
    return value;
  }

  private static Object toWire(Object value) {
    Object raw = HaraBox.unwrap(value);
    if (raw == null
        || raw instanceof String
        || raw instanceof byte[]
        || raw instanceof Number
        || raw instanceof Boolean
        || raw instanceof Throwable) {
      return raw;
    }
    if (raw instanceof Keyword keyword) return keyword.getName();
    if (raw instanceof Iterable<?> iterable) {
      ArrayList<Object> result = new ArrayList<>();
      for (Object item : iterable) result.add(toWire(item));
      return result;
    }
    throw failure("encode", "cannot encode " + raw.getClass().getCanonicalName());
  }

  private static Client client(Object value, String operation) {
    Object raw = HaraBox.unwrap(value);
    if (!(raw instanceof Client)) throw failure(operation, "expects a RESP client");
    return (Client) raw;
  }

  private static void ensureOpen(Client client, String operation) {
    if (client.closed || client.connection.socket.isClosed()) {
      throw failure(operation, "client is closed");
    }
  }

  private static void requireArity(Object[] values, int arity, String operation) {
    if (values.length != arity) {
      throw failure(operation, "expects " + arity + " argument" + (arity == 1 ? "" : "s"));
    }
  }

  private static int integer(Object value, String operation, int minimum, int maximum) {
    Object raw = HaraBox.unwrap(value);
    if (!(raw instanceof Number number)) throw failure(operation, "expects an integer");
    long result = number.longValue();
    if (result < minimum || result > maximum) {
      throw failure(operation, "integer must be between " + minimum + " and " + maximum);
    }
    return (int) result;
  }

  private static int optionInteger(
      Object options, String name, int fallback, int minimum, int maximum) {
    Object value = option(options, name);
    return value == null ? fallback : integer(value, "connect :" + name, minimum, maximum);
  }

  private static String optionName(Object options, String name, String fallback) {
    Object value = option(options, name);
    if (value == null) return fallback;
    if (value instanceof Keyword keyword) return keyword.getName();
    return String.valueOf(HaraBox.unwrap(value)).replaceFirst("^:", "");
  }

  @SuppressWarnings("rawtypes")
  private static Object option(Object options, String name) {
    if (options == null) return null;
    if (options instanceof IMapType<?, ?> map) {
      return HaraContext.lookupValue(map, Keyword.create(name));
    }
    if (options instanceof Map<?, ?> map) {
      Object value = map.get(name);
      return value == null ? map.get(Keyword.create(name)) : value;
    }
    throw failure("connect", "options must be a map");
  }

  private static HaraException failure(String operation, String message) {
    return new HaraException("std.resp.client/" + operation + " " + message);
  }

  private static final class Client {
    private final RespConnection connection;
    private final boolean bulkBytes;
    private boolean closed;

    private Client(RespConnection connection, boolean bulkBytes) {
      this.connection = connection;
      this.bulkBytes = bulkBytes;
    }

    @Override
    public String toString() {
      return closed ? "#<resp-client closed>" : "#<resp-client " + connection.socket.getRemoteSocketAddress() + ">";
    }
  }
}
