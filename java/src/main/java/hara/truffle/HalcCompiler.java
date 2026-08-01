package hara.truffle;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;

/** Build-time compiler for deterministic portable HALC module artifacts. */
final class HalcCompiler {
  private HalcCompiler() {}

  static int run(String[] args, java.io.PrintStream output, java.io.PrintStream error) {
    if ((args.length != 3 && args.length != 5) || !"--output".equals(args[1])
        || (args.length == 5 && !"--resource".equals(args[3]))) {
      error.println("compile-halc expects SOURCE --output OUTPUT [--resource ID]");
      return 2;
    }
    Path source = Path.of(args[0]);
    Path target = Path.of(args[2]);
    try {
      byte[] sourceBytes = Files.readAllBytes(source);
      Object[] forms =
          HaraLanguage.readAll(
              new String(sourceBytes, StandardCharsets.UTF_8),
              HalcArtifact.FOUNDATION_RESOURCE);
      String namespace = HalcArtifact.declaredNamespace(forms);
      String resource = args.length == 5 ? args[4] :
          ("std.foundation".equals(namespace) ? HalcArtifact.FOUNDATION_RESOURCE : args[0]);
      byte[] artifact =
          HalcArtifact.encode(namespace, resource, sourceBytes, forms);
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
      error.println("Unable to compile HALC: " + failure.getMessage());
      return 1;
    }
  }
}
