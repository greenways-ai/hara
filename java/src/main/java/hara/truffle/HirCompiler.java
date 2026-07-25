package hara.truffle;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;

/** Build-time compiler for deterministic portable HIR module artifacts. */
final class HirCompiler {
  private HirCompiler() {}

  static int run(String[] args, java.io.PrintStream output, java.io.PrintStream error) {
    if (args.length != 3 || !"--output".equals(args[1])) {
      error.println("compile-hir expects SOURCE --output OUTPUT");
      return 2;
    }
    Path source = Path.of(args[0]);
    Path target = Path.of(args[2]);
    try {
      byte[] sourceBytes = Files.readAllBytes(source);
      Object[] forms =
          HaraLanguage.readAll(
              new String(sourceBytes, StandardCharsets.UTF_8),
              HirArtifact.FOUNDATION_RESOURCE);
      String namespace = HirArtifact.declaredNamespace(forms);
      if (!"std.foundation".equals(namespace)) {
        throw new HaraException(
            "foundation HIR compiler expected std.foundation, received " + namespace);
      }
      byte[] artifact =
          HirArtifact.encode(namespace, HirArtifact.FOUNDATION_RESOURCE, sourceBytes, forms);
      Path parent = target.toAbsolutePath().getParent();
      if (parent != null) Files.createDirectories(parent);
      Files.write(target, artifact);
      output.println(
          "Compiled "
              + namespace
              + " to "
              + target
              + " ("
              + artifact.length
              + " bytes)");
      return 0;
    } catch (IOException | RuntimeException failure) {
      error.println("Unable to compile HIR: " + failure.getMessage());
      return 1;
    }
  }
}
