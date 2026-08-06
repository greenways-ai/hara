package std.lib.resp;

import hara.kernel.protocol.IRedirect;

import java.io.*;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.LinkedList;
import java.util.List;

/**
 * A lightweight RESP2 connection for servers and clients.
 *
 * <p>Effectively a complete SocketConnection client implementation.
 */
public class RespConnection implements Closeable {
  /** Implements the encoding (writing) side. */
  public static class Encoder {
    /** CRLF is used a lot. */
    private static final byte[] CRLF = new byte[] {'\r', '\n'};

    /** This stream we will write to. */
    private final OutputStream out;

    /**
     * Construct the encoder with the passed outputstream the encoder will write to.
     *
     * @param out Will be used to write all encoded data to.
     */
    public Encoder(OutputStream out) {
      this.out = out;
    }

    /**
     * Write a byte array in the "RESP Bulk String" format.
     *
     * @param value The byte array to write.
     * @throws IOException Propagated from the output stream.
     * @link https://redis.io/topics/protocol#resp-bulk-strings
     */
    public void write(byte[] value) throws IOException {
      out.write('$');
      out.write(Long.toString(value.length).getBytes(StandardCharsets.US_ASCII));
      out.write(CRLF);
      out.write(value);
      out.write(CRLF);
    }

    /**
     * Write a long value in the "RESP Integers" format.
     *
     * @param val The value to write.
     * @throws IOException Propagated from the output stream.
     * @link https://redis.io/topics/protocol#resp-integers
     */
    public void write(long val) throws IOException {
      out.write(':');
      out.write(Long.toString(val).getBytes(StandardCharsets.US_ASCII));
      out.write(CRLF);
    }

    public void write(Throwable val) throws IOException {
      out.write('-');
      out.write((val.getMessage() + "").getBytes(StandardCharsets.UTF_8));
      out.write(CRLF);
    }

    /**
     * Write a list of objects in the "RESP Arrays" format.
     *
     * @param list A list of objects that contains Strings, Longs, Integers and (recursively) Lists.
     * @throws IOException Propagated from the output stream.
     * @throws IllegalArgumentException If the list contains unencodable objects.
     * @link https://redis.io/topics/protocol#resp-arrays
     */
    public void write(List<?> list) throws IOException, IllegalArgumentException {
      out.write('*');
      out.write(Long.toString(list.size()).getBytes(StandardCharsets.US_ASCII));
      out.write(CRLF);

      for (Object o : list) {
        writeValue(o);
      }
    }

    /** Writes one RESP value. Strings are encoded as binary-safe bulk strings. */
    public void writeValue(Object value) throws IOException {
      if (value == null) {
        out.write('$');
        out.write("-1".getBytes(StandardCharsets.US_ASCII));
        out.write(CRLF);
      } else if (value instanceof byte[]) {
        write((byte[]) value);
      } else if (value instanceof String) {
        write(((String) value).getBytes(StandardCharsets.UTF_8));
      } else if (value instanceof Number) {
        write(((Number) value).longValue());
      } else if (value instanceof Boolean) {
        writeString(Boolean.toString((Boolean) value));
      } else if (value instanceof Throwable) {
        write((Throwable) value);
      } else if (value instanceof List<?>) {
        write((List<?>) value);
      } else {
        throw new IllegalArgumentException(
            "Unexpected RESP type " + value.getClass().getCanonicalName());
      }
    }

    public void writeString(String val) throws IOException {
      out.write('+');
      if (val.indexOf('\r') >= 0 || val.indexOf('\n') >= 0) {
        throw new IllegalArgumentException("RESP simple strings cannot contain CR or LF");
      }
      out.write(val.getBytes(StandardCharsets.UTF_8));
      out.write(CRLF);
    }

    public void flush() throws IOException {
      out.flush();
    }
  }

  /** Implements the parser (reader) side of protocol. */
  public static class Parser {
    public static final int DEFAULT_MAX_BULK_LENGTH = 64 * 1024 * 1024;
    public static final int DEFAULT_MAX_ARRAY_LENGTH = 1_000_000;
    public static final int DEFAULT_MAX_LINE_LENGTH = 64 * 1024;
    public static final int DEFAULT_MAX_NESTING = 128;
    /** Thrown whenever data could not be parsed. */
    @SuppressWarnings("serial")
    public static class ProtocolException extends IOException {
      public ProtocolException(String msg) {
        super(msg);
      }
    }

    /** Thrown whenever an error string is decoded. */
    @SuppressWarnings("serial")
    public static class ServerError extends IOException {
      public ServerError(String msg) {
        super(msg);
      }
    }

    /** The input stream used to read the data from. */
    private final InputStream input;
    private final int maxBulkLength;
    private final int maxArrayLength;

    /**
     * Constructor.
     *
     * @param input The stream to read the data from.
     */
    public Parser(InputStream input) {
      this(input, DEFAULT_MAX_BULK_LENGTH, DEFAULT_MAX_ARRAY_LENGTH);
    }

    public Parser(InputStream input, int maxBulkLength, int maxArrayLength) {
      if (maxBulkLength < 0 || maxArrayLength < 0) {
        throw new IllegalArgumentException("RESP parser limits must be non-negative");
      }
      this.input = input;
      this.maxBulkLength = maxBulkLength;
      this.maxArrayLength = maxArrayLength;
    }

    /**
     * Parse incoming data from the stream.
     *
     * <p>Based on each of the markers which will identify the type of data being sent, the parsing
     * is delegated to the type-specific methods.
     *
     * @return The parsed object
     * @throws IOException Propagated from the stream
     * @throws ProtocolException In case unexpected bytes are encountered.
     */
    public Object parse() throws IOException, ProtocolException {
      return parse(0);
    }

    private Object parse(int depth) throws IOException, ProtocolException {
      if (depth > DEFAULT_MAX_NESTING) {
        throw new ProtocolException("RESP nesting exceeds " + DEFAULT_MAX_NESTING);
      }
      Object ret;
      int read = this.input.read();
      switch (read) {
        case '+':
          ret = this.parseLine();
          break;
        case '-':
          ret = new ServerError(new String(this.parseLine(), StandardCharsets.UTF_8));
          break;
        case ':':
          ret = this.parseNumber();
          break;
        case '$':
          ret = this.parseBulkString();
          break;
        case '*':
          long len = this.parseNumber();
          if (len == -1) {
            ret = null;
          } else {
            if (len < -1 || len > maxArrayLength) {
              throw new ProtocolException("Unsupported array length: " + len);
            }
            List<Object> arr = new LinkedList<>();
            for (long i = 0; i < len; i++) {
              arr.add(this.parse(depth + 1));
            }
            ret = arr;
          }
          break;
        case -1:
          return null;
        default:
          throw new ProtocolException("Unexpected input: " + (byte) read);
      }

      return ret;
    }

    /**
     * Parse "RESP Bulk string" as a String object.
     *
     * @return The parsed response
     * @throws IOException Propagated from underlying stream.
     */
    private byte[] parseBulkString() throws IOException, ProtocolException {
      final long expectedLength = parseNumber();
      if (expectedLength == -1) return null;
      if (expectedLength < -1 || expectedLength > maxBulkLength) {
        throw new ProtocolException("Unsupported bulk string length: " + expectedLength);
      }
      final int numBytes = (int) expectedLength;
      final byte[] buffer = new byte[numBytes];
      int read = 0;
      while (read < expectedLength) {
        int count = input.read(buffer, read, numBytes - read);
        if (count < 0) throw new ProtocolException("Unexpected EOF in bulk string");
        read += count;
      }
      if (input.read() != '\r') {
        throw new ProtocolException("Expected CR");
      }
      if (input.read() != '\n') {
        throw new ProtocolException("Expected LF");
      }

      return buffer;
    }

    /**
     * Parse "RESP Simple String"
     *
     * @return Resultant string
     * @throws IOException Propagated from underlying stream.
     */
    private byte[] parseLine() throws IOException {
      return scanCr(1024);
    }

    private long parseNumber() throws IOException {
      try {
        return Long.parseLong(new String(scanCr(32), StandardCharsets.US_ASCII));
      } catch (NumberFormatException error) {
        throw new ProtocolException("Invalid RESP integer");
      }
    }

    private byte[] scanCr(int size) throws IOException {
      int idx = 0;
      int ch;
      byte[] buffer = new byte[size];
      while ((ch = input.read()) != '\r') {
        if (ch < 0) throw new ProtocolException("Unexpected EOF before CRLF");
        buffer[idx++] = (byte) ch;
        if (idx == size) {
          if (size >= DEFAULT_MAX_LINE_LENGTH) {
            throw new ProtocolException("RESP line exceeds " + DEFAULT_MAX_LINE_LENGTH);
          }
          size = Math.min(size * 2, DEFAULT_MAX_LINE_LENGTH);
          buffer = java.util.Arrays.copyOf(buffer, size);
        }
      }
      if (input.read() != '\n') {
        throw new ProtocolException("Expected LF");
      }

      return Arrays.copyOfRange(buffer, 0, idx);
    }
  }

  /** Used for writing the data to the server. */
  public final Encoder writer;

  /** Used for reading responses from the server. */
  public final Parser reader;

  /** An IRedirect to redirect I/O. */
  public final IRedirect redirect;

  /** Used for reading responses from the server. */
  public Socket socket;

  /**
   * Construct the connection with the specified Socket as the server connection with default buffer
   * sizes.
   *
   * @param socket Connected socket to the server.
   * @throws IOException If a socket error occurs.
   */
  public RespConnection(Socket socket) throws IOException {
    this(socket, 1 << 16, 1 << 16, null);
  }

  /**
   * Construct the connection with the specified Socket as the server connection with specified
   * buffer sizes.
   *
   * @param socket Socket to connect to
   * @param inputBufferSize buffer size in bytes for the input stream
   * @param outputBufferSize buffer size in bytes for the output stream
   * @throws IOException If a socket error occurs.
   */
  public RespConnection(Socket socket, int inputBufferSize, int outputBufferSize) throws IOException {
    this(socket, inputBufferSize, outputBufferSize, null);
  }

  public RespConnection(Socket socket, int inputBufferSize, int outputBufferSize, IRedirect redirect)
      throws IOException {
    this(
        new BufferedInputStream(socket.getInputStream(), inputBufferSize),
        new BufferedOutputStream(socket.getOutputStream(), outputBufferSize),
        redirect);
    this.socket = socket;
  }

  /**
   * Construct with the specified streams to respectively read from and write to.
   *
   * @param inputStream Read from this stream
   * @param outputStream Write to this stream
   */
  protected RespConnection(
      BufferedInputStream inputStream, BufferedOutputStream outputStream, IRedirect redirect) {
    this.reader = new Parser(inputStream);
    this.writer = new Encoder(outputStream);
    this.redirect = redirect;
  }

  /**
   * Execute a SocketConnection command and return it's result.
   *
   * @param args Command and arguments to pass into redis.
   * @param <T> The expected result type
   * @return Result of redis.
   * @throws IOException All protocol and io errors are IO exceptions.
   */
  public <T> T call(Object... args) throws IOException {
    write(args);
    return read();
  }

  /**
   * Does a blocking read to wait for redis to send data.
   *
   * @param <T> The expected result type.
   * @return Result of redis
   * @throws IOException Propagated
   */
  @SuppressWarnings("unchecked")
  public <T> T read() throws IOException {
    Object obj = reader.parse();
    if (this.redirect != null) {
      this.redirect.read(obj);
    }
    return (T) obj;
  }

  @SuppressWarnings("rawtypes")
  public void write(List args) throws IOException {
    if (this.redirect != null) {
      this.redirect.write(args);
    }
    writer.write(args);
    writer.flush();
  }

  public void write(Object... args) throws IOException {
    if (this.redirect != null) {
      this.redirect.write(args);
    }
    writer.write(Arrays.asList(args));
    writer.flush();
  }

  public void write(long val) throws IOException {
    if (this.redirect != null) {
      this.redirect.write(val);
    }
    writer.write(val);
    writer.flush();
  }

  public void write(byte[] val) throws IOException {
    if (this.redirect != null) {
      this.redirect.write(val);
    }
    writer.write(val);
    writer.flush();
  }

  public void write(Throwable t) throws IOException {
    if (this.redirect != null) {
      this.redirect.write(t);
    }
    writer.write(t);
    writer.flush();
  }

  public void writeString(String val) throws IOException {
    if (this.redirect != null) {
      this.redirect.write(val);
    }
    writer.writeString(val);
    writer.flush();
  }

  public void writeValue(Object value) throws IOException {
    if (this.redirect != null) this.redirect.write(value);
    writer.writeValue(value);
    writer.flush();
  }

  @Override
  public void close() throws IOException {
    if (this.socket != null) this.socket.close();
  }

  /** Helper class for pipelining. */
  public abstract static class Pipeline {
    /**
     * Write a new command to the server.
     *
     * @param args Command and arguments.
     * @return self for chaining
     * @throws IOException Propagated from underlying server.
     */
    public abstract Pipeline call(Object... args) throws IOException;

    /**
     * Returns an aligned list of responses for each of the calls.
     *
     * @return The responses
     * @throws IOException Propagated from underlying server.
     */
    public abstract List<Object> read() throws IOException;

    public abstract int counter();
  }

  /**
   * Create a pipeline which writes all commands to the server and only starts reading the response
   * when read() is called.
   *
   * @return A pipeline object.
   */
  public Pipeline pipeline() {
    return new Pipeline() {
      private int n = 0;

      @Override
      public int counter() {
        return n;
      }

      @Override
      public Pipeline call(Object... args) throws IOException {
        writer.write(Arrays.asList(args));
        writer.flush();
        n++;
        return this;
      }

      @Override
      public List<Object> read() throws IOException {
        List<Object> ret = new LinkedList<>();
        while (n-- > 0) {
          ret.add(reader.parse());
        }
        return ret;
      }
    };
  }
}
