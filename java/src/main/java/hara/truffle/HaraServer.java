package hara.truffle;

import std.lib.resp.RespConnection;
import java.io.IOException;
import java.net.InetSocketAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicBoolean;
import org.graalvm.nativeimage.ImageInfo;
import org.graalvm.polyglot.Value;

/** RESP listener for a shared Hara session kernel. */
public final class HaraServer implements AutoCloseable {
  public static final String DEFAULT_HOST = "127.0.0.1";
  public static final int DEFAULT_PORT = 1311;

  private static final List<String> LEGACY_COMMANDS =
      List.of(
          "HELLO",
          "AUTH",
          "PING",
          "ECHO",
          "COMMANDS",
          "INFO",
          "STATUS",
          "SESSION",
          "EVAL",
          "LOAD",
          "DOC",
          "COMPLETE",
          "INTERRUPT",
          "QUIT");

  private final String host;
  private final int requestedPort;
  private final boolean logRequests;
  private final SessionKernel kernel;
  private final boolean ownsKernel;
  private final String instanceId = UUID.randomUUID().toString();
  private final String projectRoot;
  private final Map<String, Handler> handlers = new LinkedHashMap<>();
  private ExecutorService clients;

  private final AtomicBoolean running = new AtomicBoolean();
  private ServerSocket socket;
  private Thread acceptor;

  public HaraServer() {
    this(DEFAULT_HOST, DEFAULT_PORT, false, false);
  }

  public HaraServer(String host, int port, boolean logRequests) {
    this(host, port, logRequests, false);
  }

  public HaraServer(String host, int port, boolean logRequests, boolean allowNetwork) {
    this(new SessionKernel(false, allowNetwork), host, port, logRequests, true, null);
  }

  HaraServer(SessionKernel kernel, String host, int port, boolean logRequests) {
    this(kernel, host, port, logRequests, null);
  }

  HaraServer(
      SessionKernel kernel, String host, int port, boolean logRequests, Path projectRoot) {
    this(kernel, host, port, logRequests, false, projectRoot);
  }

  private HaraServer(
      SessionKernel kernel,
      String host,
      int port,
      boolean logRequests,
      boolean ownsKernel,
      Path projectRoot) {
    this.kernel = kernel;
    this.host = host;
    this.requestedPort = port;
    this.logRequests = logRequests;
    this.ownsKernel = ownsKernel;
    this.projectRoot =
        projectRoot == null ? null : projectRoot.toAbsolutePath().normalize().toString();
    registerCoreHandlers();
  }

  /** A protocol-4 operation. Additional handlers may be registered before the server starts. */
  public interface Handler {
    String operation();

    void handle(Request request, Responder responder) throws Exception;
  }

  /** Protocol-4 request context exposed to operation handlers. */
  public static final class Request {
    private final HaraServer server;
    private final ConnectionState state;
    private final String operation;
    private final String id;
    private final List<Object> arguments;

    private Request(
        HaraServer server,
        ConnectionState state,
        String operation,
        String id,
        List<Object> arguments) {
      this.server = server;
      this.state = state;
      this.operation = operation;
      this.id = id;
      this.arguments = List.copyOf(arguments);
    }

    public String operation() {
      return operation;
    }

    public String id() {
      return id;
    }

    public List<Object> arguments() {
      return arguments;
    }

    public String argument(int index) {
      if (index < 0 || index >= arguments.size()) {
        throw new IllegalArgumentException("WRONG_NUMBER_OF_ARGUMENTS");
      }
      return text(arguments.get(index));
    }

    public String connectionId() {
      return state.connectionId;
    }

    public String session() {
      return requireSessionName(state.attached);
    }

    public void attach(String name) {
      String target = SessionKernel.normalizeName(name);
      server.requireSession(target);
      state.attached = target;
    }

    public void detach() {
      state.attached = null;
    }

    public Object eval(String source) {
      return eval(source, null, 1, 1);
    }

    public Object eval(String source, String file, int line, int column) {
      try {
        return server.requireSession(session()).eval(source, file, line, column);
      } catch (RuntimeException error) {
        throw new EvaluationException(error.getMessage(), error);
      }
    }

    public Set<String> sessionNames() {
      return server.kernel.sessionNames();
    }
  }

  /** Writes values for one protocol-4 request. Completion is added by the server. */
  public interface Responder {
    void result(Object value) throws IOException;
  }

  /** Registers or replaces a protocol-4 operation before the listener starts. */
  public synchronized void registerHandler(Handler handler) {
    if (running.get()) throw new IllegalStateException("Cannot register RESP handlers after start");
    String operation = handler.operation().toUpperCase(Locale.ROOT);
    if (operation.isBlank() || "HELLO".equals(operation)) {
      throw new IllegalArgumentException("Invalid RESP handler operation: " + operation);
    }
    handlers.put(operation, handler);
  }

  private static final class ConnectionState {
    private final String connectionId;
    private String attached = "ROOT";
    private int protocol;
    private boolean closeAfterResponse;

    private ConnectionState(String connectionId) {
      this.connectionId = connectionId;
    }
  }

  @FunctionalInterface
  private interface Operation {
    void handle(Request request, Responder responder) throws Exception;
  }

  private void register(String operation, Operation implementation) {
    registerHandler(
        new Handler() {
          @Override
          public String operation() {
            return operation;
          }

          @Override
          public void handle(Request request, Responder responder) throws Exception {
            implementation.handle(request, responder);
          }
        });
  }

  private void registerCoreHandlers() {
    register("PING", (request, responder) -> responder.result("PONG"));
    register("ECHO", (request, responder) -> responder.result(request.arguments()));
    register(
        "COMMANDS",
        (request, responder) -> {
          ArrayList<String> commands = new ArrayList<>(handlers.keySet());
          commands.add("HELLO");
          Collections.sort(commands);
          responder.result(commands);
        });
    register(
        "INFO",
        (request, responder) ->
            responder.result(info(request.connectionId(), request.state.attached, 4)));
    register(
        "STATUS",
        (request, responder) ->
            responder.result(info(request.connectionId(), request.state.attached, 4)));
    register("SESSION", this::handleSessionV4);
    register("EVAL", this::handleEvaluationV4);
    register("LOAD", this::handleEvaluationV4);
    register(
        "DOC",
        (request, responder) ->
            responder.result(valueAsWire(evaluateDocumentation(request, request.argument(0)))));
    register(
        "COMPLETE",
        (request, responder) ->
            responder.result(completions(request.session(), request.argument(0))));
    register(
        "INTERRUPT",
        (request, responder) -> {
          throw new UnsupportedOperationException("INTERRUPT_UNSUPPORTED");
        });
    register(
        "QUIT",
        (request, responder) -> {
          responder.result("BYE");
          request.state.closeAfterResponse = true;
        });
  }

  private void handleEvaluationV4(Request request, Responder responder) throws IOException {
    String source = request.argument(0);
    String file = null;
    int line = 1;
    int column = 1;
    if (request.arguments().size() > 1) {
      if ((request.arguments().size() & 1) == 0) {
        throw new IllegalArgumentException("EVAL_OPTIONS_EXPECT_KEY_VALUE_PAIRS");
      }
      for (int index = 1; index < request.arguments().size(); index += 2) {
        String key = request.argument(index).toUpperCase(Locale.ROOT);
        String value = request.argument(index + 1);
        switch (key) {
          case "FILE":
            file = value;
            break;
          case "LINE":
            line = positiveInteger(value, "LINE");
            break;
          case "COLUMN":
            column = positiveInteger(value, "COLUMN");
            break;
          default:
            throw new IllegalArgumentException("UNKNOWN_EVAL_OPTION " + key);
        }
      }
    }
    responder.result(display(request.eval(source, file, line, column)));
  }

  private static int positiveInteger(String value, String option) {
    try {
      int parsed = Integer.parseInt(value);
      if (parsed < 1) throw new NumberFormatException();
      return parsed;
    } catch (NumberFormatException error) {
      throw new IllegalArgumentException(option + "_EXPECTS_POSITIVE_INTEGER");
    }
  }

  private void handleSessionV4(Request request, Responder responder) throws IOException {
    String sub = request.argument(0).toUpperCase(Locale.ROOT);
    switch (sub) {
      case "NEW":
        String created = SessionKernel.normalizeName(request.argument(1));
        kernel.create(created);
        responder.result(created);
        break;
      case "LIST":
        ArrayList<String> names = new ArrayList<>(kernel.sessionNames());
        Collections.sort(names);
        responder.result(names);
        break;
      case "ATTACH":
        request.attach(request.argument(1));
        responder.result(request.session());
        break;
      case "DETACH":
        request.detach();
        responder.result("DETACHED");
        break;
      case "INFO":
        String target =
            request.arguments().size() > 1
                ? SessionKernel.normalizeName(request.argument(1))
                : request.session();
        responder.result(requireSession(target).info());
        break;
      case "FILESYSTEM":
        String filesystemSession = SessionKernel.normalizeName(request.argument(1));
        kernel.attachFilesystem(filesystemSession, Path.of(request.argument(2)));
        responder.result(requireSession(filesystemSession).info());
        break;
      case "CLOSE":
      case "KILL":
        String closed = SessionKernel.normalizeName(request.argument(1));
        kernel.closeSession(closed);
        if (closed.equals(request.state.attached)) request.detach();
        responder.result(true);
        break;
      default:
        throw new IllegalArgumentException("UNKNOWN_SESSION_COMMAND " + sub);
    }
  }


  public synchronized HaraServer start() throws IOException {
    if (running.get()) return this;
    socket = new ServerSocket();
    socket.bind(new InetSocketAddress(host, requestedPort));
    clients =
        ImageInfo.inImageRuntimeCode()
            ? Executors.newCachedThreadPool()
            : Executors.newVirtualThreadPerTaskExecutor();
    running.set(true);
    acceptor = new Thread(this::acceptLoop, "hara-resp-acceptor");
    acceptor.setDaemon(false);
    acceptor.start();
    return this;
  }

  public boolean isRunning() {
    return running.get();
  }

  public int port() {
    ServerSocket current = socket;
    return current == null ? requestedPort : current.getLocalPort();
  }

  public Set<String> sessionNames() {
    return kernel.sessionNames();
  }

  public synchronized void stop() {
    if (!running.getAndSet(false)) return;
    try {
      if (socket != null) socket.close();
    } catch (IOException ignored) {
    }
    if (clients != null) clients.shutdownNow();
    if (acceptor != null) acceptor.interrupt();
  }

  @Override
  public void close() {
    stop();
    if (ownsKernel) kernel.close();
  }

  private void acceptLoop() {
    while (running.get()) {
      try {
        Socket client = socket.accept();
        clients.submit(() -> handle(client));
      } catch (IOException error) {
        if (running.get()) error.printStackTrace();
      }
    }
  }

  private void handle(Socket client) {
    ConnectionState state =
        new ConnectionState(Integer.toHexString(System.identityHashCode(client)));
    try (Socket ignored = client;
        RespConnection conn = new RespConnection(client)) {
      while (!client.isClosed()) {
        Object raw = conn.read();
        if (raw == null) return;
        List<Object> values = values(raw);
        if (values.isEmpty()) {
          conn.write(new RuntimeException("EMPTY_COMMAND"));
          continue;
        }
        List<String> request = strings(raw);
        if (logRequests) System.err.println("REQUEST " + state.connectionId + " " + request);
        String command = text(values.get(0)).toUpperCase(Locale.ROOT);
        try {
          if ("HELLO".equals(command)) {
            int requested = values.size() > 1 ? protocol(text(values.get(1))) : 3;
            state.protocol = Math.min(requested, 4);
            conn.write(hello(state.connectionId, state.protocol));
          } else if (state.protocol == 4) {
            handleV4(conn, state, command, values);
            if (state.closeAfterResponse) return;
          } else if ("PING".equals(command)) {
            conn.writeString("PONG");
          } else if ("ECHO".equals(command)) {
            conn.write(request.subList(1, request.size()));
          } else if ("COMMANDS".equals(command) || "HELP".equals(command)) {
            conn.write(LEGACY_COMMANDS);
          } else if ("INFO".equals(command) || "STATUS".equals(command)) {
            conn.write(info(state.connectionId, state.attached, state.protocol));
          } else if ("SESSION".equals(command)) {
            state.attached =
                sessionCommand(conn, request, state.attached, state.protocol == 3);
          } else if ("EVAL".equals(command) || "LOAD".equals(command)) {
            state.attached =
                evaluate(conn, request, state.attached, state.protocol == 3, command);
          } else if ("DOC".equals(command)) {
            evaluateDoc(conn, request, state.attached, state.protocol == 3);
          } else if ("COMPLETE".equals(command)) {
            evaluateComplete(conn, request, state.attached, state.protocol == 3);
          } else if ("QUIT".equals(command)) {
            conn.writeString("BYE");
            return;
          } else if ("INTERRUPT".equals(command)) {
            conn.write(new RuntimeException("INTERRUPT_UNSUPPORTED"));
          } else {
            conn.write(new RuntimeException("UNKNOWN_COMMAND " + command));
          }
        } catch (Throwable error) {
          if (state.protocol == 4) writeErrorV4(conn, values, error);
          else writeError(conn, request, state.protocol == 3, error);
        }
      }
    } catch (IOException ignored) {
    }
  }

  private void handleV4(
      RespConnection conn, ConnectionState state, String command, List<Object> values)
      throws Exception {
    if (values.size() < 2) throw new IllegalArgumentException("MISSING_REQUEST_ID");
    String id = text(values.get(1));
    if (id.isBlank()) throw new IllegalArgumentException("MISSING_REQUEST_ID");
    Handler handler = handlers.get(command);
    if (handler == null) throw new UnknownOperationException(command);
    Request request =
        new Request(this, state, command, id, values.subList(2, values.size()));
    handler.handle(
        request,
        value -> conn.write(Arrays.asList("RESULT", id, valueAsWire(value))));
    conn.write(Arrays.asList("DONE", id, "OK"));
  }

  private List<Object> hello(String connection, int protocol) {
    return Arrays.asList(
        "SERVER",
        "HARA",
        "VERSION",
        "0.1.0",
        "PROTO",
        (long) protocol,
        "RUNTIME",
        "TRUFFLE",
        "CONNECTION",
        connection,
        "INSTANCE",
        instanceId,
        "PROJECT",
        projectRoot,
        "CAPABILITIES",
        commands());
  }

  private List<String> commands() {
    ArrayList<String> result = new ArrayList<>(handlers.keySet());
    result.add("HELLO");
    Collections.sort(result);
    return result;
  }

  private List<Object> info(String connection, String attached, int protocol) {
    return Arrays.asList(
        "SERVER",
        "HARA",
        "HOST",
        host,
        "PORT",
        (long) port(),
        "CONNECTION",
        connection,
        "PROTO",
        (long) protocol,
        "INSTANCE",
        instanceId,
        "PROJECT",
        projectRoot,
        "SESSION",
        attached,
        "SESSIONS",
        (long) kernel.size());
  }

  private String sessionCommand(
      RespConnection conn, List<String> request, String attached, boolean negotiated) throws IOException {
    require(request, 2);
    String sub = request.get(1).toUpperCase(java.util.Locale.ROOT);
    switch (sub) {
      case "NEW":
        require(request, 3);
        String created = SessionKernel.normalizeName(request.get(2));
        kernel.create(created);
        respond(conn, negotiated, request, created);
        return attached;
      case "LIST":
        List<String> names = new ArrayList<>(kernel.sessionNames());
        Collections.sort(names);
        respond(conn, negotiated, request, names);
        return attached;
      case "ATTACH":
        require(request, 3);
        String target = SessionKernel.normalizeName(request.get(2));
        requireSession(target);
        respond(conn, negotiated, request, target);
        return target;
      case "DETACH":
        respond(conn, negotiated, request, "DETACHED");
        return null;
      case "INFO":
        String targetInfo = request.size() > 2 ? SessionKernel.normalizeName(request.get(2)) : attached;
        SessionKernel.Session sessionInfo = requireSession(targetInfo);
        respond(conn, negotiated, request, sessionInfo.info());
        return attached;
      case "FILESYSTEM":
        require(request, 4);
        String filesystemSession = SessionKernel.normalizeName(request.get(2));
        kernel.attachFilesystem(filesystemSession, Path.of(request.get(3)));
        respond(conn, negotiated, request, requireSession(filesystemSession).info());
        return attached;
      case "CLOSE":
      case "KILL":
        require(request, 3);
        String targetClose = SessionKernel.normalizeName(request.get(2));
        kernel.closeSession(targetClose);
        respond(conn, negotiated, request, true);
        return targetClose.equals(attached) ? null : attached;
      default:
        throw new IllegalArgumentException("UNKNOWN_SESSION_COMMAND " + sub);
    }
  }

  private String evaluate(
      RespConnection conn, List<String> request, String attached, boolean negotiated, String command)
      throws IOException {
    String requestId = negotiated ? requireValue(request, 1) : "";
    String sessionName;
    String source;
    if (negotiated) {
      sessionName = requireSessionName(attached);
      source = requireValue(request, 2);
    } else {
      sessionName = SessionKernel.normalizeName(requireValue(request, 1));
      source = requireValue(request, 2);
    }
    SessionKernel.Session session = requireSession(sessionName);
    Object result = session.eval(source);
    if (negotiated) {
      conn.write(Arrays.asList("RESULT", requestId, display(result)));
      conn.write(Arrays.asList("DONE", requestId, "OK"));
    } else {
      conn.writeString(display(result));
    }
    return attached;
  }

  private void evaluateDoc(
      RespConnection conn, List<String> request, String attached, boolean negotiated)
      throws IOException {
    String requestId = negotiated ? requireValue(request, 1) : "";
    String symbol = negotiated ? requireValue(request, 2) : requireValue(request, 1);
    Object result = evaluateDocumentation(requireSessionName(attached), symbol);
    respond(conn, negotiated, request, result, requestId);
  }

  private void evaluateComplete(
      RespConnection conn, List<String> request, String attached, boolean negotiated)
      throws IOException {
    String requestId = negotiated ? requireValue(request, 1) : "";
    String prefix = negotiated ? requireValue(request, 2) : requireValue(request, 1);
    respond(
        conn,
        negotiated,
        request,
        completions(requireSessionName(attached), prefix),
        requestId);
  }

  private Object evaluateDocumentation(Request request, String symbol) {
    validateSymbol(symbol);
    return request.eval(documentationSource(symbol));
  }

  private Object evaluateDocumentation(String session, String symbol) {
    validateSymbol(symbol);
    return requireSession(session).eval(documentationSource(symbol));
  }

  private static String documentationSource(String symbol) {
    return "[\"SYMBOL\" \""
        + symbol
        + "\" \"DOC\" (get (meta #'"
        + symbol
        + ") :doc) \"ARGLISTS\" (get (meta #'"
        + symbol
        + ") :arglists) \"FILE\" (get (meta #'"
        + symbol
        + ") :file) \"LINE\" (get (meta #'"
        + symbol
        + ") :line) \"COLUMN\" (get (meta #'"
        + symbol
        + ") :column)]";
  }

  private List<String> completions(String session, String prefix) {
    Object result = requireSession(session).eval("(current-symbols)");
    List<String> matches = new ArrayList<>();
    if (result instanceof Value) {
      Value values = (Value) result;
      for (long i = 0; i < values.getArraySize(); i++) {
        String name = values.getArrayElement(i).asString();
        if (name.toLowerCase(java.util.Locale.ROOT)
            .contains(prefix.toLowerCase(java.util.Locale.ROOT))) matches.add(name);
      }
    }
    Collections.sort(matches);
    return matches;
  }

  private void respond(RespConnection conn, boolean negotiated, List<String> request, Object value)
      throws IOException {
    respond(conn, negotiated, request, value, negotiated ? requireValue(request, 1) : "");
  }

  private void respond(
      RespConnection conn, boolean negotiated, List<String> request, Object value, String id)
      throws IOException {
    if (negotiated) conn.write(Arrays.asList("RESULT", id, valueAsWire(value)));
    else conn.write(valueAsWire(value));
  }

  private void writeError(
      RespConnection conn, List<String> request, boolean negotiated, Throwable error)
      throws IOException {
    String message = error.getMessage() == null ? error.toString() : error.getMessage();
    if (negotiated) {
      String id = request.size() > 1 ? request.get(1) : "";
      conn.write(Arrays.asList("ERROR", id, "REQUEST_ERROR", message));
      if (!id.isEmpty()) conn.write(Arrays.asList("DONE", id, "ERROR"));
    } else {
      conn.write(new RuntimeException("ERR " + message));
    }
  }

  private void writeErrorV4(RespConnection conn, List<Object> request, Throwable error)
      throws IOException {
    String id = request.size() > 1 ? text(request.get(1)) : "";
    String message = error.getMessage() == null ? error.toString() : error.getMessage();
    conn.write(Arrays.asList("ERROR", id, errorCode(error, message), message));
    conn.write(Arrays.asList("DONE", id, "ERROR"));
  }

  private static String errorCode(Throwable error, String message) {
    if (error instanceof UnknownOperationException) return "UNKNOWN_OP";
    if (error instanceof UnsupportedOperationException) return "UNSUPPORTED";
    if (error instanceof EvaluationException) return "EVAL_ERROR";
    if (message.startsWith("NO_SESSION") || message.startsWith("NO_SESSION_ATTACHED")) {
      return "NO_SESSION";
    }
    if (error instanceof IllegalArgumentException) {
      if (message.contains("Unbound") || message.contains("Syntax") || message.contains("parse")) {
        return "EVAL_ERROR";
      }
      return "BAD_REQUEST";
    }
    return "INTERNAL_ERROR";
  }

  private static Object valueAsWire(Object value) {
    if (value == null) return null;
    if (value instanceof Value) {
      Value guest = (Value) value;
      if (guest.isNull()) return null;
      if (guest.hasArrayElements()) {
        List<Object> result = new ArrayList<>();
        for (long index = 0; index < guest.getArraySize(); index++)
          result.add(valueAsWire(guest.getArrayElement(index)));
        return result;
      }
      if (guest.isBoolean()) return guest.asBoolean();
      if (guest.isString()) return guest.asString();
      if (guest.fitsInLong()) return guest.asLong();
      if (guest.fitsInDouble()) return java.lang.Double.toString(guest.asDouble());
      return guest.toString();
    }
    if (value instanceof Map<?, ?>) {
      List<Object> result = new ArrayList<>();
      for (Map.Entry<?, ?> entry : ((Map<?, ?>) value).entrySet())
        result.add(Arrays.asList(String.valueOf(entry.getKey()), valueAsWire(entry.getValue())));
      return result;
    }
    if (value instanceof Iterable<?>) {
      List<Object> result = new ArrayList<>();
      for (Object item : (Iterable<?>) value) result.add(valueAsWire(item));
      return result;
    }
    return value instanceof Number || value instanceof Boolean ? value.toString() : value;
  }

  private static String display(Object value) {
    if (value == null) return "nil";
    if (value instanceof Value) {
      Value v = (Value) value;
      if (v.isNull()) return "nil";
      return v.toString();
    }
    return String.valueOf(value);
  }

  private SessionKernel.Session requireSession(String name) {
    return kernel.require(name);
  }

  private static String requireSessionName(String name) {
    if (name == null) throw new IllegalArgumentException("NO_SESSION_ATTACHED");
    return name;
  }

  private static void validateSymbol(String symbol) {
    if (!symbol.matches("[A-Za-z0-9_.*?!+/<>=:-]+"))
      throw new IllegalArgumentException("INVALID_SYMBOL");
  }

  private static void require(List<String> request, int count) {
    if (request.size() < count) throw new IllegalArgumentException("WRONG_NUMBER_OF_ARGUMENTS");
  }

  private static String requireValue(List<String> request, int index) {
    require(request, index + 1);
    return request.get(index);
  }

  private static List<String> strings(Object value) {
    if (!(value instanceof List<?>)) return List.of(String.valueOf(value));
    List<String> result = new ArrayList<>();
    for (Object item : (List<?>) value) {
      if (item instanceof byte[]) result.add(new String((byte[]) item, StandardCharsets.UTF_8));
      else result.add(String.valueOf(item));
    }
    return result;
  }

  @SuppressWarnings("unchecked")
  private static List<Object> values(Object value) {
    if (value instanceof List<?>) return (List<Object>) value;
    return List.of(value);
  }

  private static String text(Object value) {
    if (value instanceof byte[]) return new String((byte[]) value, StandardCharsets.UTF_8);
    return String.valueOf(value);
  }

  private static int protocol(String value) {
    try {
      return Integer.parseInt(value) >= 4 ? 4 : 3;
    } catch (NumberFormatException error) {
      throw new IllegalArgumentException("INVALID_PROTOCOL " + value);
    }
  }

  private static final class UnknownOperationException extends IllegalArgumentException {
    private UnknownOperationException(String operation) {
      super("UNKNOWN_COMMAND " + operation);
    }
  }

  private static final class EvaluationException extends RuntimeException {
    private EvaluationException(String message, Throwable cause) {
      super(message, cause);
    }
  }

}
