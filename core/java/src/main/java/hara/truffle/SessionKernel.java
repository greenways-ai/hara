package hara.truffle;

import hara.lang.protocol.Constant;
import hara.lang.protocol.IApplicable;
import hara.lang.protocol.IComponent;
import hara.lang.protocol.IContext;
import hara.lang.protocol.IInvokeIn;
import hara.lang.protocol.IMetadata;
import java.io.IOException;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicInteger;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.HostAccess;
import org.graalvm.polyglot.PolyglotException;
import org.graalvm.polyglot.Source;
import org.graalvm.polyglot.Value;
import org.graalvm.polyglot.io.IOAccess;

/** Owns the runtime contexts shared by local and RESP clients. */
final class SessionKernel implements AutoCloseable {
  private final boolean allowFile;
  private final boolean allowNetwork;
  private final boolean allowProcess;
  private final HaraProject project;
  private final ConcurrentHashMap<String, Session> sessions = new ConcurrentHashMap<>();

  SessionKernel(boolean allowFile, boolean allowNetwork) {
    this(allowFile, allowNetwork, false);
  }

  SessionKernel(boolean allowFile, boolean allowNetwork, boolean allowProcess) {
    this(allowFile, allowNetwork, allowProcess, null);
  }

  SessionKernel(
      boolean allowFile, boolean allowNetwork, boolean allowProcess, HaraProject project) {
    this.allowFile = allowFile;
    this.allowNetwork = allowNetwork;
    this.allowProcess = allowProcess;
    this.project = project;
    sessions.put("ROOT", new Session("ROOT", allowFile, allowNetwork, allowProcess, project));
  }

  Session root() {
    return require("ROOT");
  }

  Session require(String name) {
    Session session = sessions.get(name);
    if (session == null) throw new IllegalArgumentException("NO_SESSION " + name);
    return session;
  }

  synchronized Session create(String value) {
    String name = normalizeName(value);
    if (sessions.containsKey(name)) throw new IllegalArgumentException("SESSION_EXISTS " + name);
    Session session = new Session(name, allowFile, allowNetwork, allowProcess, project);
    sessions.put(name, session);
    return session;
  }

  void attachFilesystem(String session, Path root) {
    if (!allowFile) throw new IllegalArgumentException("FILE_ACCESS_DENIED");
    require(session).attachFilesystem(root);
  }

  synchronized void closeSession(String value) {
    String name = normalizeName(value);
    if ("ROOT".equals(name)) throw new IllegalArgumentException("ROOT_CANNOT_CLOSE");
    Session removed = sessions.remove(name);
    if (removed == null) throw new IllegalArgumentException("NO_SESSION " + name);
    removed.close();
  }

  Set<String> sessionNames() {
    return Collections.unmodifiableSet(sessions.keySet());
  }

  int size() {
    return sessions.size();
  }

  @Override
  public synchronized void close() {
    for (Session session : sessions.values()) session.close();
    sessions.clear();
  }

  static String normalizeName(String value) {
    if (value == null || value.isEmpty() || !value.matches("[A-Za-z0-9_.-]+"))
      throw new IllegalArgumentException("INVALID_SESSION_NAME");
    return value;
  }

  static final class Session
      implements AutoCloseable, IContext, IComponent, IApplicable, IInvokeIn {
    private final String name;
    private final boolean allowFile;
    private final boolean allowNetwork;
    private final boolean allowProcess;
    private final HaraProject project;
    private Context context;
    private Path filesystemRoot;
    private final AtomicInteger activeEvaluations = new AtomicInteger();
    private final java.util.concurrent.atomic.AtomicBoolean active =
        new java.util.concurrent.atomic.AtomicBoolean(true);

    static final class SessionMetadata implements IMetadata {
      final String name;
      final String namespace;
      final String state;
      final String filesystem;

      SessionMetadata(String name, String namespace, String state, String filesystem) {
        this.name = name;
        this.namespace = namespace;
        this.state = state;
        this.filesystem = filesystem;
      }

      @Override
      public Constant.MetaType getMetatype() {
        return Constant.MetaType.MAP;
      }
    }

    private Session(
        String name,
        boolean allowFile,
        boolean allowNetwork,
        boolean allowProcess,
        HaraProject project) {
      this.name = name;
      this.allowFile = allowFile;
      this.allowNetwork = allowNetwork;
      this.allowProcess = allowProcess;
      this.project = project;
      context = createContext(null);
    }

    private Context createContext(Path root) {
      IOAccess.Builder io =
          IOAccess.newBuilder().allowHostSocketAccess(allowNetwork);
      if (root == null) {
        io.allowHostFileAccess(allowFile || project != null);
      } else {
        io.allowHostFileAccess(false).fileSystem(new HaraMountedFileSystem(root));
      }
      Context.Builder builder =
          Context.newBuilder(HaraLanguage.ID)
          .allowCreateProcess(allowProcess)
          .allowIO(io.build());
      if (project != null && root == null) builder.currentWorkingDirectory(project.root());
      if (project != null && project.hasCapability("jvm/reflection")) {
        builder.allowHostAccess(HostAccess.ALL).allowHostClassLookup(name -> true);
      }
      return builder.build();
    }

    private void requireActive() {
      if (!active.get()) throw new IllegalStateException("SESSION_CLOSED " + name);
    }

    void attachFilesystem(Path root) {
      requireActive();
      if (activeEvaluations.get() != 0) throw new IllegalArgumentException("SESSION_BUSY " + name);
      Path normalized = root.toAbsolutePath().normalize();
      if (!java.nio.file.Files.isDirectory(normalized)) {
        throw new IllegalArgumentException("FILESYSTEM_NOT_FOUND " + normalized);
      }
      Context replacement = createContext(normalized);
      synchronized (this) {
        if (activeEvaluations.get() != 0) {
          replacement.close(true);
          throw new IllegalArgumentException("SESSION_BUSY " + name);
        }
        Context previous = context;
        context = replacement;
        filesystemRoot = normalized;
        previous.close(true);
      }
    }

    String name() {
      return name;
    }

    Value eval(String source) {
      return eval(source, null, 1, 1);
    }

    Object evalTransfer(String source) {
      return transferValue(eval(source));
    }

    private static Object transferValue(Value value) {
      if (value.isNull()) return null;
      if (value.isBoolean()) return value.asBoolean();
      if (value.isString()) return value.asString();
      if (value.fitsInLong()) return value.asLong();
      if (value.fitsInDouble()) return value.asDouble();
      String display = value.toString();
      if (display.contains("#'")
          || display.contains("#atom")
          || display.contains("#<")
          || display.contains("#object")
          || display.contains("#array")
          || display.contains("#bytes")
          || display.contains("@")) {
        throw new IllegalArgumentException("SESSION_TRANSFER_REJECTED " + display);
      }
      if (value.hasIterator() && display.startsWith("#{")) {
        java.util.LinkedHashSet<Object> transferred = new java.util.LinkedHashSet<>();
        Value iterator = value.getIterator();
        while (iterator.hasIteratorNextElement()) {
          transferred.add(transferValue(iterator.getIteratorNextElement()));
        }
        return HaraPersistentValues.normalize(transferred);
      }
      if (value.hasArrayElements()) {
        java.util.ArrayList<Object> transferred = new java.util.ArrayList<>();
        for (long index = 0; index < value.getArraySize(); index++) {
          transferred.add(transferValue(value.getArrayElement(index)));
        }
        return HaraPersistentValues.normalize(transferred);
      }
      if (value.hasHashEntries()) {
        java.util.LinkedHashMap<Object, Object> transferred = new java.util.LinkedHashMap<>();
        Value entries = value.getHashEntriesIterator();
        while (entries.hasIteratorNextElement()) {
          Value entry = entries.getIteratorNextElement();
          transferred.put(
              transferValue(entry.getArrayElement(0)),
              transferValue(entry.getArrayElement(1)));
        }
        return HaraPersistentValues.normalize(transferred);
      }
      if (value.hasIterator()) {
        throw new IllegalArgumentException("SESSION_TRANSFER_REJECTED " + display);
      }
      try {
        Object[] forms = HaraLanguage.readAll(display, "<session-transfer>");
        if (forms.length != 1) {
          throw new IllegalArgumentException("SESSION_TRANSFER_REJECTED " + display);
        }
        return forms[0];
      } catch (RuntimeException error) {
        if (error instanceof IllegalArgumentException
            && error.getMessage() != null
            && error.getMessage().startsWith("SESSION_TRANSFER_REJECTED")) {
          throw error;
        }
        throw new IllegalArgumentException(
            "SESSION_TRANSFER_REJECTED "
                + display
                + " ("
                + error.getClass().getSimpleName()
                + ": "
                + error.getMessage()
                + ")",
            error);
      }
    }

    Value eval(String source, String file, int line, int column) {
      requireActive();
      activeEvaluations.incrementAndGet();
      try {
        synchronized (this) {
          if (file == null || file.isBlank()) return context.eval(HaraLanguage.ID, source);
          int safeLine = Math.max(1, line);
          int safeColumn = Math.max(1, column);
          StringBuilder contextual = new StringBuilder(source.length() + safeLine + safeColumn);
          contextual.append("\n".repeat(safeLine - 1));
          contextual.append(" ".repeat(safeColumn - 1));
          contextual.append(source);
          Source contextualSource =
              Source.newBuilder(HaraLanguage.ID, contextual.toString(), file).build();
          return context.eval(contextualSource);
        }
      } catch (IOException error) {
        throw new IllegalArgumentException("Unable to construct Hara source: " + error.getMessage(), error);
      } catch (PolyglotException error) {
        throw new IllegalArgumentException(error.getMessage(), error);
      } finally {
        activeEvaluations.decrementAndGet();
      }
    }

    synchronized String currentNamespace() {
      Value value = eval("(current-namespace)");
      return value.isString() ? value.asString() : value.toString();
    }

    synchronized List<String> currentSymbols() {
      Value values = eval("(current-symbols)");
      List<String> result = new ArrayList<>();
      for (long index = 0; index < values.getArraySize(); index++) {
        result.add(values.getArrayElement(index).asString());
      }
      return result;
    }

    List<Object> info() {
      return List.of(
          "NAME", name,
          "STATE", "RUNNING",
          "FILESYSTEM", filesystemRoot == null ? "HOST" : filesystemRoot.toString());
    }

    @Override
    public Object call(Object... args) {
      requireActive();
      if (args == null || args.length != 1 || !(args[0] instanceof String)) {
        throw new IllegalArgumentException("SESSION_CALL_EXPECTS_SOURCE " + name);
      }
      return evalTransfer((String) args[0]);
    }

    @Override
    public IMetadata getProps() {
      return metadata();
    }

    @Override
    public IMetadata getStatus() {
      return metadata();
    }

    private SessionMetadata metadata() {
      boolean running = active.get();
      return new SessionMetadata(
          name,
          running ? currentNamespace() : null,
          running ? (activeEvaluations.get() == 0 ? "idle" : "busy") : "closed",
          filesystemRoot == null ? null : filesystemRoot.toString());
    }

    @Override
    public boolean isStarted() {
      return active.get();
    }

    @Override
    public boolean isStopped() {
      return !active.get();
    }

    @Override
    public IComponent start() {
      if (!active.get()) throw new IllegalStateException("SESSION_CLOSED " + name);
      return this;
    }

    @Override
    public IComponent stop() {
      close();
      return this;
    }

    @Override
    public Object applyDefault() {
      return this;
    }

    @Override
    public Object applyIn(Object runtime, Object[] args) {
      requireActive();
      if (!(runtime instanceof IContext)) {
        throw new IllegalArgumentException("SESSION_APPLY_EXPECTS_CONTEXT " + name);
      }
      return ((IContext) runtime).call(args == null ? new Object[0] : args);
    }

    @Override
    public Object transformIn(Object runtime, Object[] args) {
      return args;
    }

    @Override
    public Object transformOut(Object runtime, Object[] args, Object value) {
      return value;
    }

    @Override
    public Object invokeIn(IContext context, Object... args) {
      return applyIn(context, args);
    }

    @Override
    public synchronized void close() {
      if (active.compareAndSet(true, false)) context.close(true);
    }
  }
}
