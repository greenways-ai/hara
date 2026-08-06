package hara.kernel.builtin;

import hara.lang.base.Ex;
import hara.kernel.base.Module;
import hara.kernel.base.Reflect;
import hara.kernel.protocol.IRuntime;
import hara.lang.base.Fn;
import hara.lang.protocol.IFn;
import hara.lang.base.Iter;
import hara.lang.base.primitive.Array;
import hara.lang.data.Keyword;
import hara.lang.data.Set;

import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Iterator;
import java.util.function.Function;

@SuppressWarnings({"unchecked", "rawtypes"})
public interface BuiltinInterop {
  @Module.Fn(name = "class:constructors")
  public static Constructor[] classConstructors(Class cls) {
    return cls.getDeclaredConstructors();
  }

  @Module.Fn(name = "class:fields")
  public static Field[] classFields(Class cls) {
    return cls.getDeclaredFields();
  }

  @Module.Fn(name = "class", rt = true)
  public static Class classFor(IRuntime rt, String name) {
    return rt.classFor(name);
  }

  @Module.Fn(name = "class:methods")
  public static Method[] classMethods(Class cls) {
    return cls.getDeclaredMethods();
  }

  @Module.Fn(name = "class:inner")
  public static Class[] classInner(Class cls) {
    return cls.getDeclaredClasses();
  }

  @Module.Fn(name = "class:static-methods")
  public static Set<String> classStaticMethods(Class cls) {
    Iterator<Method> methods = Iter.iter(cls.getMethods());
    return BuiltinCollection.toSet(
        Iter.map(
            Iter.filter(methods, (m) -> Modifier.isStatic(m.getModifiers())), (m) -> m.getName()));
  }

  @Module.Fn(name = "class:static-fields")
  public static Iterator<String> classStaticFields(Class cls) {
    Iterator<Field> fields = Iter.iter(cls.getFields());
    return Iter.map(
        Iter.filter(fields, (m) -> Modifier.isStatic(m.getModifiers())), (m) -> m.getName());
  }

  @Module.Fn(name = "invoke:new", vargs = true)
  public static <R, ITR> Object invokeNew(Class c, ITR args) {
    return Reflect.invokeConstructor(c, Iter.toArray(args));
  }

  @Module.Fn(name = "invoke", vargs = true)
  public static <R, ITR> Object invokeObj(Object o, String method, ITR args) {
    return Reflect.invokeInstanceMethod(o, method, Iter.toArray(args));
  }

  @Module.Fn(name = "invoke:get")
  public static <R> Object invokeGet(Object o, String name) {
    return Reflect.getInstanceField(o, name);
  }

  @Module.Fn(name = "invoke:set")
  public static <R> Object invokeSet(Object o, String name, Object val) {
    return Reflect.setInstanceField(o, name, val);
  }

  @Module.Fn(name = "invoke:static", vargs = true)
  public static <R, ITR> Object invokeStatic(Class c, String method, ITR args) {
    return Reflect.invokeStaticMethod(c, method, Iter.toArray(args));
  }

  @Module.Fn(name = "invoke:fn")
  public static <R, ITR> IFn invokeFn(Class c, String method) {
    var lu = classStaticMethods(c);
    if (lu.has(method)) {
      return Fn.toFnVargs(
          BuiltinStruct.hashMap(
              Array.objects(BuiltinBasic.keyword("name"), c.getName() + "/" + method)),
          (Function) args -> (R) Reflect.invokeStaticMethod(c, method, Iter.toArray(args)));
    } else {
      throw new Ex.Info(
          "Method not found: " + method,
          BuiltinStruct.hashMap(Array.objects(BuiltinBasic.keyword("options"), lu)));
    }
  }

  @Module.Fn(name = "invoke:get-static", vargs = true)
  public static <R> Object invokeGetStatic(Class c, String name) {
    return Reflect.getStaticField(c, name);
  }

  @Module.Fn(name = "invoke:set-static", vargs = true)
  public static <R> Object invokeSetStatic(Class c, String name, Object val) {
    return Reflect.setStaticField(c, name, val);
  }
}
