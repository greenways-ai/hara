package hara.truffle;

import java.io.IOException;
import java.net.URI;
import java.nio.channels.SeekableByteChannel;
import java.nio.file.AccessMode;
import java.nio.file.CopyOption;
import java.nio.file.DirectoryStream;
import java.nio.file.LinkOption;
import java.nio.file.OpenOption;
import java.nio.file.Path;
import java.nio.file.Files;
import java.nio.file.attribute.FileAttribute;
import java.util.Map;
import java.util.Set;
import org.graalvm.polyglot.io.FileSystem;

/** Confines a Truffle session's public filesystem to one host directory. */
final class HaraMountedFileSystem implements FileSystem {
  private final Path root;
  private final FileSystem delegate = FileSystem.newDefaultFileSystem();

  HaraMountedFileSystem(Path root) {
    this.root = root.toAbsolutePath().normalize();
  }

  Path root() {
    return root;
  }

  private Path mounted(Path path) throws IOException {
    Path normalized = path.toAbsolutePath().normalize();
    if (normalized.startsWith(root)) return normalized;
    String guest = path.normalize().toString();
    while (guest.startsWith("/") || guest.startsWith("\\")) guest = guest.substring(1);
    Path resolved = root.resolve(guest).normalize();
    if (!resolved.startsWith(root)) throw new IOException("file/denied");
    return resolved;
  }

  /**
   * Rejects paths that escape through a symlink. For a path that does not exist yet, the nearest
   * existing ancestor is checked before the delegate is allowed to create it.
   */
  private Path confined(Path path) throws IOException {
    Path candidate = mounted(path);
    Path rootReal = root.toRealPath();
    Path existing = candidate;
    while (existing != null && !Files.exists(existing, LinkOption.NOFOLLOW_LINKS)) {
      existing = existing.getParent();
    }
    if (existing == null || !existing.toRealPath().startsWith(rootReal)) {
      throw new IOException("file/denied");
    }
    if (Files.exists(candidate) && !candidate.toRealPath().startsWith(rootReal)) {
      throw new IOException("file/denied");
    }
    return candidate;
  }

  @Override
  public Path parsePath(URI uri) {
    return parsePath(Path.of(uri).toString());
  }

  @Override
  public Path parsePath(String path) {
    String guest = path == null ? "" : path;
    while (guest.startsWith("/") || guest.startsWith("\\")) guest = guest.substring(1);
    Path resolved = root.resolve(guest).normalize();
    if (!resolved.startsWith(root)) throw new IllegalArgumentException("file/denied");
    return resolved;
  }

  @Override
  public void checkAccess(Path path, Set<? extends AccessMode> modes, LinkOption... options)
      throws IOException {
    delegate.checkAccess(mounted(path), modes, options);
  }

  @Override
  public void createDirectory(Path path, FileAttribute<?>... attributes) throws IOException {
    delegate.createDirectory(confined(path), attributes);
  }

  @Override
  public void delete(Path path) throws IOException {
    delegate.delete(confined(path));
  }

  @Override
  public SeekableByteChannel newByteChannel(
      Path path, Set<? extends OpenOption> options, FileAttribute<?>... attributes)
      throws IOException {
    return delegate.newByteChannel(confined(path), options, attributes);
  }

  @Override
  public DirectoryStream<Path> newDirectoryStream(
      Path path, DirectoryStream.Filter<? super Path> filter) throws IOException {
    return delegate.newDirectoryStream(confined(path), filter);
  }

  @Override
  public Path toAbsolutePath(Path path) {
    try {
      return mounted(path);
    } catch (IOException error) {
      throw new IllegalArgumentException(error.getMessage(), error);
    }
  }

  @Override
  public Path toRealPath(Path path, LinkOption... options) throws IOException {
    Path real = delegate.toRealPath(mounted(path), options);
    if (!real.startsWith(root.toRealPath())) throw new IOException("file/denied");
    return real;
  }

  @Override
  public Map<String, Object> readAttributes(
      Path path, String attributes, LinkOption... options) throws IOException {
    return delegate.readAttributes(confined(path), attributes, options);
  }

  @Override
  public void setAttribute(Path path, String attribute, Object value, LinkOption... options)
      throws IOException {
    delegate.setAttribute(confined(path), attribute, value, options);
  }

  @Override
  public void copy(Path source, Path target, CopyOption... options) throws IOException {
    delegate.copy(confined(source), confined(target), options);
  }

  @Override
  public void move(Path source, Path target, CopyOption... options) throws IOException {
    delegate.move(confined(source), confined(target), options);
  }

  @Override
  public Path getTempDirectory() {
    return root.resolve(".tmp");
  }
}
