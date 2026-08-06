package hara.lang.base.primitive;

import hara.lang.base.Ex;
import hara.lang.base.Iter;
import hara.lang.data.types.ObjFn;
import hara.lang.protocol.IFn;
import hara.lang.protocol.IMetadata;

import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.util.Arrays;
import java.util.Iterator;
import java.util.LinkedList;
import java.util.function.BiFunction;
import java.util.function.Function;
import java.util.function.Supplier;

public interface Reflect {

  public class InstanceField<R> extends ObjFn implements IFn<R, Object, Object> {
    final Field _mh;

    public InstanceField(Field mh) {
      this(null, mh);
    }

    public InstanceField(IMetadata meta, Field mh) {
      super(meta);
      _mh = mh;
    }

    @Override
    public Function<Object, R> getArg1() {
      return (o) -> {
        try {
          return (R) _mh.get(o);
        } catch (IllegalArgumentException | IllegalAccessException t) {
          throw Ex.Sneaky(t);
        }
      };
    }

    @Override
    public BiFunction<Object, Object, R> getArg2() {
      return (o, arg) -> {
        try {
          _mh.set(o, arg);
        } catch (IllegalArgumentException | IllegalAccessException t) {
          throw Ex.Sneaky(t);
        }
        return (R) arg;
      };
    }
  }

  public class InstanceMethod<R> extends ObjFn implements IFn<R, Object, Object> {
    final Method _mh;

    public InstanceMethod(IMetadata meta, Method mh) {
      super(meta);
      _mh = mh;
    }

    public InstanceMethod(Method mh) {
      this(null, mh);
    }

    public void checkArgs(int size) {
      if (size != _mh.getParameterCount()) {
        throw new Ex.Arity(size, "Only " + _mh.getParameterCount() + " Args supported");
      }
    }

    @Override
    public Function<Object, R> getArg1() {
      checkArgs(1);
      return (arg) -> invokeHandle(arg, Arrays.asList());
    }

    @Override
    public BiFunction<Object, Object, R> getArg2() {
      checkArgs(2);
      return (a1, a2) -> invokeHandle(a1, Arrays.asList(a2));
    }

    @Override
    public Function<Object, R> getArgN() {
      return (vargs) -> {
        Iterator it = Iter.iter(vargs);
        var o = it.next();
        java.util.List args = Iter.toArrayList(it);
        checkArgs(args.size());
        return invokeHandle(o, args);
      };
    }

    public R invokeHandle(Object o, java.util.List args) {
      try {
        return (R) _mh.invoke(o, args);
      } catch (Throwable t) {
        throw Ex.Sneaky(t);
      }
    }
  }

  public class StaticConstructor<R> extends ObjFn implements IFn<R, Object, Object> {
    final Constructor _mh;

    public StaticConstructor(Constructor mh) {
      this(null, mh);
    }

    public StaticConstructor(IMetadata meta, Constructor mh) {
      super(meta);
      _mh = mh;
    }

    public void checkArgs(int size) {
      if (size != _mh.getParameterCount()) {
        throw new Ex.Arity(size, "Only " + _mh.getParameterCount() + " Args supported");
      }
    }

    @Override
    public Supplier<R> getArg0() {
      checkArgs(0);
      return () -> invokeHandle(new LinkedList());
    }

    @Override
    public Function<Object, R> getArg1() {
      checkArgs(1);
      return (arg) -> invokeHandle(Arrays.asList(arg));
    }

    @Override
    public BiFunction<Object, Object, R> getArg2() {
      checkArgs(2);
      return (a1, a2) -> invokeHandle(Arrays.asList(a1, a2));
    }

    @Override
    public Function<Object, R> getArgN() {
      return (vargs) -> {
        java.util.List args = Iter.toArrayList(Iter.iter(vargs));
        checkArgs(args.size());
        return invokeHandle(args);
      };
    }

    public R invokeHandle(java.util.List args) {
      try {
        return (R) _mh.newInstance(args.toArray());
      } catch (Throwable t) {
        throw Ex.Sneaky(t);
      }
    }
  }

  public class StaticField<R> extends ObjFn implements IFn<R, Object, Object> {
    final Field _mh;

    public StaticField(Field mh) {
      this(null, mh);
    }

    public StaticField(IMetadata meta, Field mh) {
      super(meta);
      _mh = mh;
    }

    @Override
    public Supplier<R> getArg0() {
      return () -> {
        try {
          return (R) _mh.get(null);
        } catch (IllegalArgumentException | IllegalAccessException t) {
          throw Ex.Sneaky(t);
        }
      };
    }

    @Override
    public Function<Object, R> getArg1() {
      return (arg) -> {
        try {
          _mh.set(null, arg);
        } catch (IllegalArgumentException | IllegalAccessException t) {
          throw Ex.Sneaky(t);
        }
        return (R) arg;
      };
    }
  }

  public class StaticMethod<R> extends ObjFn implements IFn<R, Object, Object> {
    final Method _mh;

    public StaticMethod(IMetadata meta, Method mh) {
      super(meta);
      _mh = mh;
    }

    public StaticMethod(Method mh) {
      this(null, mh);
    }

    public void checkArgs(int size) {
      if (size != _mh.getParameterCount()) {
        throw new Ex.Arity(size, "Only " + _mh.getParameterCount() + " Args supported");
      }
    }

    @Override
    public Supplier<R> getArg0() {
      checkArgs(0);
      return () -> invokeHandle(new LinkedList());
    }

    @Override
    public Function<Object, R> getArg1() {
      checkArgs(1);
      return (arg) -> invokeHandle(Arrays.asList(arg));
    }

    @Override
    public BiFunction<Object, Object, R> getArg2() {
      checkArgs(2);
      return (a1, a2) -> invokeHandle(Arrays.asList(a1, a2));
    }

    @Override
    public Function<Object, R> getArgN() {
      return (vargs) -> {
        java.util.List args = Iter.toArrayList(Iter.iter(vargs));
        checkArgs(args.size());
        return invokeHandle(args);
      };
    }

    public R invokeHandle(java.util.List args) {
      try {
        return (R) _mh.invoke(null, args);
      } catch (Throwable t) {
        throw Ex.Sneaky(t);
      }
    }
  }
}
