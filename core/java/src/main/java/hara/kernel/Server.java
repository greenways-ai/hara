package hara.kernel;

import hara.kernel.protocol.IRedirect;
import hara.lang.base.Ex;
import hara.lang.protocol.IComponent;
import hara.lang.protocol.IMetadata;

import java.io.IOException;
import java.net.ServerSocket;
import java.net.Socket;
import java.util.List;
import java.util.stream.Collectors;

@SuppressWarnings("rawtypes")
public class Server implements IComponent {

  final hara.lang.base.primitive.RefCache<Thread, Conn> CLIENTS =
      new hara.lang.base.primitive.RefCache<>();

  public final Foundation _F;
  public final String _key;
  public final int _port;
  public final IRedirect _redirect;
  public ServerSocket _socket;
  public Thread _thread;

  public Server(Foundation F, String key) {
    this(F, key, Foundation.DEFAULT_PORT, null);
  }

  public Server(Foundation F, String key, int port) {
    this(F, key, port, null);
  }

  public Server(Foundation F, String key, int port, IRedirect redirect) {
    _F = F;
    _key = key;
    _port = port;
    _redirect = redirect;
  }

  public long count() {
    return CLIENTS.count();
  }

  @Override
  public IComponent start() {
    try {
      _socket = new ServerSocket(_port);
      System.out.println("PORT OPENED: " + _port);
      Loop loop = new Loop(this, _socket);
      _thread = new Thread(loop);
      _thread.start();
    } catch (Throwable t) {
      throw Ex.Sneaky(t);
    }
    return this;
  }

  @Override
  public IComponent stop() {
    _thread.interrupt();
    _thread = null;
    for (var entry : CLIENTS.getLookup().entrySet()) {
      try {
        entry.getValue().get().close();
        entry.getKey().interrupt();
      } catch (IOException e) {
      }
    }
    if (_redirect != null) {
      try {
        _redirect.close();
      } catch (IOException e) {
        e.printStackTrace();
      }
    }
    return this;
  }

  @Override
  public boolean isStopped() {
    return !isStarted();
  }

  @Override
  public boolean isStarted() {
    return (_thread != null) && _thread.isAlive();
  }

  @Override
  public IMetadata getStatus() {
    return null;
  }

  @Override
  public IMetadata getProps() {
    return null;
  }

  private class Loop implements Runnable {

    Server _instance;
    ServerSocket _socket;

    public Loop(Server instance, ServerSocket socket) {
      _instance = instance;
      _socket = socket;
    }

    @Override
    public void run() {
      try {
        while (!_socket.isClosed()) {
          Socket s = _socket.accept();
          Handler h = new Handler(_instance, s);
          Thread th = new Thread(h);
          h.setThread(th);
          _instance.CLIENTS.register(th, h._conn);
          th.start();
        }
      } catch (IOException e) {
      }
    }
  }

  // ClientHandler class
  private class Handler implements Runnable {

    public final Server _instance;
    public final Conn _conn;
    private Thread _thread;

    // constructor
    public Handler(Server instance, Socket s) throws IOException {
      _instance = instance;
      _conn = new Conn(s, 1 << 16, 1 << 16, _instance._redirect);
    }

    public void setThread(Thread thread) {
      _thread = thread;
    }

    private void writeReturn(Conn conn, Object ret) throws IOException {
      if (ret instanceof List) {
        conn.write((List) ret);
      } else if (ret instanceof String) {
        conn.writeString((String) ret);
      } else if (ret instanceof Long) {
        conn.write(((Long) ret).longValue());
      } else if (ret instanceof Throwable) {
        conn.write((Throwable) ret);
      } else if (ret instanceof byte[]) {
        conn.write((byte[]) ret);
      } else {
        conn.writeString(ret.toString());
      }
    }

    @SuppressWarnings("unchecked")
    @Override
    public void run() {
      try {
        while (!_conn.socket.isClosed()) {
          try {
            Object obj = _conn.read();
            if (obj instanceof List) {
              List args =
                  (List)
                      ((List) obj)
                          .stream().map(b -> new String((byte[]) b)).collect(Collectors.toList());
              var ret = Foundation.runCommand(_instance._F, args);
              writeReturn(_conn, ret);
            } else {
              _conn.write("NOT PROCESSED: " + obj);
            }
          } catch (Throwable t) {
            _conn.write("ERR: " + t.getMessage());
            t.printStackTrace();
          }
        }
      } catch (Throwable t) {
        _instance.CLIENTS.deregister(_thread);
      }
    }
  }
}
