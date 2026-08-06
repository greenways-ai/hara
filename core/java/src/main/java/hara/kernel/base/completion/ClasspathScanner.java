package hara.kernel.base.completion;

import java.io.File;
import java.io.IOException;
import java.net.URI;
import java.net.URL;
import java.nio.file.*;
import java.util.Enumeration;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.jar.JarEntry;
import java.util.jar.JarFile;
import java.util.stream.Stream;

public class ClasspathScanner {

  private final PackageTree tree;
  private final Set<String> scannedPaths = ConcurrentHashMap.newKeySet();

  public ClasspathScanner(PackageTree tree) {
    this.tree = tree;
  }

  public void scanBackground() {
    Thread t = new Thread(this::scan, "ClasspathScanner");
    t.setDaemon(true);
    t.start();
  }

  public void scan() {
    scanSystemClasspath();
    scanJrtModules();
  }

  public void scan(URL[] urls) {
    if (urls == null) return;
    for (URL url : urls) {
      try {
        if ("file".equals(url.getProtocol())) {
          File f = new File(url.toURI());
          String path = f.getAbsolutePath();
          if (scannedPaths.add(path)) {
            if (f.isDirectory()) {
              scanDirectory(f, "");
            } else if (f.getName().endsWith(".jar")) {
              scanJar(f);
            }
          }
        }
      } catch (Exception e) {
        // ignore malformed URLs or other errors
      }
    }
  }

  private void scanSystemClasspath() {
    String cp = System.getProperty("java.class.path");
    if (cp == null) return;

    String[] entries = cp.split(File.pathSeparator);
    for (String entry : entries) {
      File f = new File(entry);
      if (!f.exists()) continue;

      String path = f.getAbsolutePath();
      if (scannedPaths.add(path)) {
        if (f.isDirectory()) {
          scanDirectory(f, "");
        } else if (entry.toLowerCase().endsWith(".jar")) {
          scanJar(f);
        }
      }
    }
  }

  private void scanDirectory(File dir, String packageName) {
    File[] files = dir.listFiles();
    if (files == null) return;

    for (File f : files) {
      if (f.isDirectory()) {
        String newPackage = packageName.isEmpty() ? f.getName() : packageName + "." + f.getName();
        scanDirectory(f, newPackage);
      } else if (f.getName().endsWith(".class")) {
        String className = f.getName().substring(0, f.getName().length() - 6); // remove .class
        String fullClassName = packageName.isEmpty() ? className : packageName + "." + className;
        tree.add(fullClassName);
      }
    }
  }

  private void scanJar(File jarFile) {
    try (JarFile jar = new JarFile(jarFile)) {
      Enumeration<JarEntry> entries = jar.entries();
      while (entries.hasMoreElements()) {
        JarEntry entry = entries.nextElement();
        if (entry.isDirectory()) continue;

        String name = entry.getName();
        if (name.endsWith(".class")) {
          String className = name.substring(0, name.length() - 6).replace('/', '.');
          tree.add(className);
        }
      }
    } catch (IOException e) {
      // Ignore bad jars
    }
  }

  private void scanJrtModules() {
    try {
      FileSystem fs = FileSystems.getFileSystem(URI.create("jrt:/"));
      Path modules = fs.getPath("/modules");
      if (!Files.exists(modules)) return;

      try (Stream<Path> stream = Files.walk(modules)) {
        stream.forEach(
            path -> {
              String s = path.toString();
              if (s.endsWith(".class") && !s.contains("module-info.class")) {
                int first = s.indexOf('/', 1); // after /modules
                int second = s.indexOf('/', first + 1); // after module name

                if (second != -1) {
                  String classPath = s.substring(second + 1);
                  String className =
                      classPath.substring(0, classPath.length() - 6).replace('/', '.');
                  tree.add(className);
                }
              }
            });
      }
    } catch (Throwable t) {
      // Failed to access jrt fs
    }
  }
}
