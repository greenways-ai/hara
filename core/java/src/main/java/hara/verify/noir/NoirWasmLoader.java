package hara.verify.noir;

import java.util.ServiceLoader;
import java.util.concurrent.CompletableFuture;

/** Host capability for compiling, proving, and verifying Noir through WASM. */
public interface NoirWasmLoader {
  String id();

  boolean available();

  CompletableFuture<NoirArtifact> compile(NoirProgram program);

  CompletableFuture<NoirProof> prove(NoirArtifact artifact, String inputsJson);

  CompletableFuture<Boolean> verify(NoirArtifact artifact, NoirProof proof);

  static NoirWasmLoader discover() {
    for (NoirWasmLoader loader : ServiceLoader.load(NoirWasmLoader.class)) {
      if (loader.available()) return loader;
    }
    return Unavailable.INSTANCE;
  }

  final class Unavailable implements NoirWasmLoader {
    private static final Unavailable INSTANCE = new Unavailable();

    private Unavailable() {}

    @Override
    public String id() {
      return "unavailable";
    }

    @Override
    public boolean available() {
      return false;
    }

    @Override
    public CompletableFuture<NoirArtifact> compile(NoirProgram program) {
      return unavailable("compile");
    }

    @Override
    public CompletableFuture<NoirProof> prove(NoirArtifact artifact, String inputsJson) {
      return unavailable("prove");
    }

    @Override
    public CompletableFuture<Boolean> verify(NoirArtifact artifact, NoirProof proof) {
      return unavailable("verify");
    }

    private static <T> CompletableFuture<T> unavailable(String operation) {
      CompletableFuture<T> result = new CompletableFuture<>();
      result.completeExceptionally(
          new IllegalStateException(
              "noir/" + operation + " is unavailable: install a capability-scoped NoirWasmLoader"));
      return result;
    }
  }
}
