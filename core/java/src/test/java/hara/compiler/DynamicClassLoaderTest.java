package hara.compiler;

import junit.framework.TestCase;
import org.objectweb.asm.ClassWriter;
import org.objectweb.asm.MethodVisitor;
import org.objectweb.asm.Opcodes;

public class DynamicClassLoaderTest extends TestCase {

  public void testConstructor() {
    ClassLoader parentClassLoader = getClass().getClassLoader();
    DynamicClassLoader classLoader = new DynamicClassLoader(parentClassLoader);
    assertNotNull(classLoader);
    assertEquals(parentClassLoader, classLoader.getParent());
  }

  public void testDefineClass() throws Exception {
    ClassLoader parentClassLoader = getClass().getClassLoader();
    DynamicClassLoader classLoader = new DynamicClassLoader(parentClassLoader);

    String className = "MyTestClass";

    // Generate class using ASM
    ClassWriter cw = new ClassWriter(0);
    cw.visit(
        Opcodes.V1_8,
        Opcodes.ACC_PUBLIC + Opcodes.ACC_SUPER,
        className,
        null,
        "java/lang/Object",
        null);

    // Constructor
    MethodVisitor mv = cw.visitMethod(Opcodes.ACC_PUBLIC, "<init>", "()V", null, null);
    mv.visitCode();
    mv.visitVarInsn(Opcodes.ALOAD, 0);
    mv.visitMethodInsn(Opcodes.INVOKESPECIAL, "java/lang/Object", "<init>", "()V", false);
    mv.visitInsn(Opcodes.RETURN);
    mv.visitMaxs(1, 1);
    mv.visitEnd();

    cw.visitEnd();

    byte[] bytecode = cw.toByteArray();

    Class<?> definedClass = classLoader.defineClass(className, bytecode);
    assertNotNull(definedClass);
    assertEquals(className, definedClass.getName());
    assertEquals(classLoader, definedClass.getClassLoader());
  }
}
