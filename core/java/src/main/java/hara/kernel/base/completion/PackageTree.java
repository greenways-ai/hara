package hara.kernel.base.completion;

import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicBoolean;

public class PackageTree {

  public static class Node {
    final ConcurrentHashMap<String, Node> children = new ConcurrentHashMap<>();
    final AtomicBoolean isClass = new AtomicBoolean(false);

    public void add(String[] parts, int index) {
      if (index >= parts.length) {
        isClass.set(true);
        return;
      }

      String part = parts[index];
      children.computeIfAbsent(part, k -> new Node()).add(parts, index + 1);
    }

    public void collect(String prefix, List<String> results) {
      if (isClass.get()) {
        results.add(prefix); // Add the class itself
        // But if it has children (inner classes), we continue?
        // Usually, in completion, if we match a class "java.lang.String", we are done for that
        // branch unless we want static members.
        // For now, let's just list the class name.
      }

      // If we are at a package level (children exist), we might want to suggest the next segment.
      // But typically, suggest() logic drives the traversal.
    }
  }

  private final Node root = new Node();

  public void add(String className) {
    if (className == null || className.isEmpty()) return;
    // Handle nested classes with $
    // java.util.Map$Entry -> java.util.Map.Entry ?
    // Or keep them as Map$Entry?
    // Usually in Java code we use Map.Entry.
    // Let's replace $ with .
    String normalized = className.replace('$', '.');
    String[] parts = normalized.split("\\.");
    root.add(parts, 0);
  }

  public List<String> suggest(String prefix) {
    List<String> results = new ArrayList<>();

    String[] parts = prefix.split("\\.", -1);

    Node current = root;
    String matchedPrefix = "";

    // Traverse to the deepest complete node
    for (int i = 0; i < parts.length - 1; i++) {
      String part = parts[i];
      current = current.children.get(part);
      if (current == null) {
        return results; // Path not found
      }
      matchedPrefix += part + ".";
    }

    String lastPart = parts[parts.length - 1];

    // Now filter children of current node that start with lastPart
    for (Map.Entry<String, Node> entry : current.children.entrySet()) {
      String key = entry.getKey();
      if (key.startsWith(lastPart)) {
        // Check if it's a class or a package (has children)
        // If it's a class, we add the full name.
        // If it's a package, we add the full name + "."?
        // Usually completion just gives the next segment.

        // JLine expects the full string to replace the word, OR just the suffix if configured
        // differently.
        // Based on previous Main.java logic, it seems to expect the full token.

        String candidate = matchedPrefix + key;
        if (entry.getValue().isClass.get()) {
          results.add(candidate);
        } else if (!entry.getValue().children.isEmpty()) {
          results.add(candidate + ".");
        }
      }
    }

    Collections.sort(results);
    return results;
  }
}
