package hara.kernel.protocol;

import java.io.Closeable;
import java.io.IOException;

public interface IRedirect extends Closeable {

  void read(Object obj) throws IOException;

  void write(Object obj) throws IOException;
}
