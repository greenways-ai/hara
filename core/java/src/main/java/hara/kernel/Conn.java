package hara.kernel;

import hara.kernel.protocol.IRedirect;
import java.io.IOException;
import java.net.Socket;

/**
 * @deprecated Use {@link std.lib.resp.RespConnection}. This compatibility type will be removed in
 *     a future release.
 */
@Deprecated
public class Conn extends std.lib.resp.RespConnection {
  public Conn(Socket socket) throws IOException {
    super(socket);
  }

  public Conn(Socket socket, int inputBufferSize, int outputBufferSize) throws IOException {
    super(socket, inputBufferSize, outputBufferSize);
  }

  public Conn(
      Socket socket, int inputBufferSize, int outputBufferSize, IRedirect redirect)
      throws IOException {
    super(socket, inputBufferSize, outputBufferSize, redirect);
  }
}
