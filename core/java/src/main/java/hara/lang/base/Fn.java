package hara.lang.base;

import hara.lang.base.fn.*;
import hara.lang.base.primitive.Reflect;
import hara.lang.protocol.IFn;
import hara.lang.protocol.IMetadata;

import java.lang.invoke.MethodHandle;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.function.*;

@SuppressWarnings({"rawtypes", "unchecked"})
public interface Fn {

  public enum UnitType {
    COMPLEX,
    FILTER,
    MAP,
    MAPCAT
  }

  public static <FN> IFn toFn(FN f) {
    return toFn(null, f);
  }

  public static IFn toFn(IMetadata meta, BiFunction f) {
    return new Fn2(meta, f);
  }

  public static IFn toFn(IMetadata meta, BiPredicate p) {
    return new Pred2(meta, p);
  }

  public static IFn toFn(IMetadata meta, Consumer c) {
    return new Consume1(meta, c);
  }

  public static <FN> IFn toFn(IMetadata meta, FN f) {
    if (f instanceof IFn) {
      return (IFn) f;
    } else if (f instanceof Supplier) {
      return toFn(meta, (Supplier) f);
    } else if (f instanceof Function) {
      return toFn(meta, (Function) f);
    } else if (f instanceof Consumer) {
      return toFn(meta, (Consumer) f);
    } else if (f instanceof Predicate) {
      return toFn(meta, (Predicate) f);
    } else if (f instanceof BiFunction) {
      return toFn(meta, (BiFunction) f);
    } else if (f instanceof BiPredicate) {
      return toFn(meta, (BiPredicate) f);
    } else if (f instanceof MethodHandle) {
      return toFn(meta, (MethodHandle) f);
    } else if (f instanceof Method) {
      return toFn(meta, (Method) f);
    } else {
      throw new Ex.Unsupported();
    }
  }

  public static IFn toFn(IMetadata meta, Function f) {
    return new Fn1(meta, f);
  }

  public static IFn toFn(IMetadata meta, Predicate p) {
    return new Pred1(meta, p);
  }

  public static IFn toFn(IMetadata meta, Supplier f) {
    return new Fn0(meta, f);
  }

  public static IFn toFn(IMetadata meta, Method f) {
    if (Modifier.isStatic(f.getModifiers())) {
      return new Reflect.StaticMethod(meta, f);
    } else {
      return new Reflect.InstanceMethod(meta, f);
    }
  }

  public static IFn toFn(IMetadata meta, Field f) {
    if (Modifier.isStatic(f.getModifiers())) {
      return new Reflect.StaticField(meta, f);
    } else {
      return new Reflect.InstanceField(meta, f);
    }
  }

  public static IFn toFn(IMetadata meta, Constructor f) {
    return new Reflect.StaticConstructor(meta, f);
  }

  public static <FN> IFn toFnVargs(IMetadata meta, Function f) {
    return new FnVargs(meta, f);
  }

  public static <FN> IFn toFnVargs(Function f) {
    return new FnVargs(null, f);
  }
}
