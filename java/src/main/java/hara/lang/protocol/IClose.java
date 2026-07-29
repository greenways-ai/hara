package hara.lang.protocol;

/** Explicitly closes a resource or traversal cursor. */
public interface IClose extends AutoCloseable {
  @Override
  void close() throws Exception;
}
