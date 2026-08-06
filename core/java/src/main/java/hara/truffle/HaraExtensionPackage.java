package hara.truffle;

import java.io.IOException;
import java.io.InputStream;
import java.net.URISyntaxException;
import java.net.URL;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.LinkedHashSet;

/** A validated extension descriptor and its colocated provider artifacts. */
final class HaraExtensionPackage {
  private static final int MAX_MODULE_BYTES = 64 * 1024 * 1024;

  private final HaraExtensionManifest manifest;
  private final URL descriptor;

  HaraExtensionPackage(HaraExtensionManifest manifest, URL descriptor) {
    this.manifest = manifest;
    this.descriptor = descriptor;
  }

  HaraExtensionManifest manifest() {
    return manifest;
  }

  URL descriptor() {
    return descriptor;
  }

  URL resolve(String relative) {
    try {
      return new URL(descriptor, relative);
    } catch (IOException error) {
      throw new HaraException(
          "extension/asset-invalid: " + manifest.namespace() + "/" + relative);
    }
  }

  Path file(String relative) {
    URL url = resolve(relative);
    if (!"file".equals(url.getProtocol())) {
      throw new HaraException(
          "extension/target-unavailable: process modules must be installed as files: " + url);
    }
    try {
      Path packageRoot = Path.of(descriptor.toURI()).getParent().toRealPath();
      Path resolved = Path.of(url.toURI()).toRealPath();
      if (!resolved.startsWith(packageRoot)) {
        throw new HaraException("extension/path-denied: " + relative);
      }
      return resolved;
    } catch (IOException | URISyntaxException error) {
      throw new HaraException(
          "extension/asset-unavailable: "
              + manifest.namespace()
              + "/"
              + relative
              + " ("
              + error.getMessage()
              + ")");
    }
  }

  void validateDeclaredFiles() {
    LinkedHashSet<String> paths = new LinkedHashSet<>(manifest.assets());
    if (manifest.module() != null) paths.add(manifest.module());
    manifest.targets().values().forEach(target -> paths.add(target.module()));
    for (String path : paths) {
      URL asset = resolve(path);
      try (InputStream ignored = asset.openStream()) {
        // Opening is sufficient here; provider-specific readers enforce size limits.
      } catch (IOException error) {
        throw new HaraException(
            "extension/asset-unavailable: " + manifest.namespace() + "/" + path);
      }
    }
  }

  byte[] moduleBytes() {
    if (manifest.module() == null) {
      throw new HaraException("extension/module-unavailable: " + manifest.namespace());
    }
    URL module = resolve(manifest.module());
    try {
      byte[] bytes;
      try (InputStream input = module.openStream()) {
        bytes = input.readNBytes(MAX_MODULE_BYTES + 1);
      }
      if (bytes.length > MAX_MODULE_BYTES) {
        throw new HaraException("extension/module-too-large: " + module);
      }
      return bytes;
    } catch (IOException error) {
      throw new HaraException(
          "extension/module-unavailable: "
              + manifest.namespace()
              + "/"
              + manifest.module()
              + " ("
              + error.getMessage()
              + ")");
    }
  }
}
