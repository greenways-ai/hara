package hara.truffle;

import java.util.concurrent.CompletableFuture;

/** Runtime-selected provider behind a generated extension namespace. */
interface HaraExtensionRuntime extends AutoCloseable {
  boolean asynchronous();

  Object invoke(String name, Object[] values);

  CompletableFuture<Object> invokeAsync(String name, Object[] values);

  @Override
  void close();
}
