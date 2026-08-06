package hara.compiler;

public class DynamicClassLoader extends ClassLoader {

  public DynamicClassLoader(ClassLoader parent) {
    super(parent);
  }

  public Class<?> defineClass(String name, byte[] b) {
    return defineClass(name, b, 0, b.length);
  }
}
