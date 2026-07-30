package hara.truffle;

import java.io.IOException;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicInteger;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.PolyglotException;
import org.graalvm.polyglot.Source;
import org.graalvm.polyglot.Value;
import org.graalvm.polyglot.io.IOAccess;

/** Owns the runtime contexts shared by local and RESP clients. */
final class HaraSessionBroker implements AutoCloseable {
  private final boolean allowFile;
  private final boolean allowNetwork;
  private final boolean allowProcess;
  private final ConcurrentHashMap<String, HaraSession> sessions = new ConcurrentHashMap<>();

  HaraSessionBroker(boolean allowFile, boolean allowNetwork) {
    this(allowFile, allowNetwork, false);
  }

  HaraSessionBroker(boolean allowFile, boolean allowNetwork, boolean allowProcess) {
    this.allowFile = allowFile;
    this.allowNetwork = allowNetwork;
    this.allowProcess = allowProcess;
    sessions.put("ROOT", new HaraSession("ROOT", allowFile, allowNetwork, allowProcess));
  }

  HaraSession root() {
    return require("ROOT");
  }

  HaraSession require(String name) {
    HaraSession session = sessions.get(name);
    if (session == null) throw new IllegalArgumentException("NO_SESSION " + name);
    return session;
  }

  synchronized HaraSession create(String value) {
    String name = normalizeName(value);
    if (sessions.containsKey(name)) throw new IllegalArgumentException("SESSION_EXISTS " + name);
    HaraSession session = new HaraSession(name, allowFile, allowNetwork, allowProcess);
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
    HaraSession removed = sessions.remove(name);
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
    for (HaraSession session : sessions.values()) session.close();
    sessions.clear();
  }

  static String normalizeName(String value) {
    if (value == null || value.isEmpty() || !value.matches("[A-Za-z0-9_.-]+"))
      throw new IllegalArgumentException("INVALID_SESSION_NAME");
    return value;
  }

  static final class HaraSession implements AutoCloseable {
    private final String name;
    private final boolean allowFile;
    private final boolean allowNetwork;
    private final boolean allowProcess;
    private Context context;
    private Path filesystemRoot;
    private final AtomicInteger activeEvaluations = new AtomicInteger();

    private HaraSession(
        String name, boolean allowFile, boolean allowNetwork, boolean allowProcess) {
      this.name = name;
      this.allowFile = allowFile;
      this.allowNetwork = allowNetwork;
      this.allowProcess = allowProcess;
      context = createContext(null);
    }

    private Context createContext(Path root) {
      IOAccess.Builder io =
          IOAccess.newBuilder().allowHostSocketAccess(allowNetwork);
      if (root == null) {
        io.allowHostFileAccess(allowFile);
      } else {
        io.allowHostFileAccess(false).fileSystem(new HaraMountedFileSystem(root));
      }
      return Context.newBuilder(HaraLanguage.ID)
          .allowCreateProcess(allowProcess)
          .allowIO(io.build())
          .build();
    }

    void attachFilesystem(Path root) {
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
    public synchronized void close() {
      context.close(true);
    }
  }
}
