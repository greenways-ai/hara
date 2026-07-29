package hara.truffle;

import hara.lang.data.types.ISequentialLookupType;
import hara.lang.data.types.ISetType;
import hara.lang.data.List;
import hara.lang.protocol.*;
import java.util.Arrays;
import java.util.Iterator;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.function.Consumer;

/** Compatibility adapters from existing Java protocol interfaces to Hara protocol dispatch. */
public final class HaraJavaAdapters {
  private HaraJavaAdapters() {}

  public static void install(HaraContext context) {
    installIFn(context.ifnProtocol());
    installLookup(context.defineProtocol("ILookup", Map.of("lookup", -1)));
    installAssoc(context.defineProtocol("IAssoc", Map.of("assoc", 3)));
    installCount(context.defineProtocol("ICount", Map.of("count", 1)));
    installConj(context.defineProtocol("IConj", Map.of("conj", 2)));
    installFind(context.defineProtocol("IFind", Map.of("find", 2)));
    installEquality(context.defineProtocol("IEquality", Map.of("equality", 2)));
    installHash(context.defineProtocol("IHash", Map.of("hash", 1)));
    installMetadata(context.defineProtocol("IObjType", metadataMethods()));
    installDeref(context.defineProtocol("IDeref", Map.of("deref", 1)));
    installDerefTimeout(context.defineProtocol("IDerefTimeout", Map.of("deref-timeout", 3)));
    installNth(context.defineProtocol("INth", Map.of("nth", 2)));
    installEmpty(context.defineProtocol("IEmpty", Map.of("empty", 1)));
    installDisplay(context.defineProtocol("IDisplay", Map.of("display", 1)));
    installCollection(context.defineProtocol("IColl", collectionMethods()));
    installCons(context.defineProtocol("ICons", Map.of("cons", 2)));
    installDissoc(context.defineProtocol("IDissoc", Map.of("dissoc", 2)));
    installIndexed(context.defineProtocol("IIndexed", Map.of("index-of", 2)));
    installIndexedKV(
        context.defineProtocol("IIndexedKV", Map.of("index-of-key", 2, "index-of-val", 2)));
    installPeekFirst(context.defineProtocol("IPeekFirst", Map.of("peek-first", 1)));
    installPeekLast(context.defineProtocol("IPeekLast", Map.of("peek-last", 1)));
    installPopFirst(context.defineProtocol("IPopFirst", Map.of("pop-first", 1)));
    installPopLast(context.defineProtocol("IPopLast", Map.of("pop-last", 1)));
    installPushFirst(context.defineProtocol("IPushFirst", Map.of("push-first", 2)));
    installPushLast(context.defineProtocol("IPushLast", Map.of("push-last", 2)));
    installRanged(
        context.defineProtocol("IRanged", Map.of("range-max", 1, "range-min", 1)));
    installRealize(context.defineProtocol("IRealize", Map.of("realized?", 1, "realize", 1)));
    installReset(context.defineProtocol("IReset", Map.of("reset", 2)));
    installConversion(
        context.defineProtocol("IToMutable", Map.of("to-mutable", 1)),
        context.defineProtocol("IToPersistent", Map.of("to-persistent", 1)));
    installValidate(context.defineProtocol("IValidate", Map.of("validate", 2, "validator", 1)));
    installWatch(
        context.defineProtocol(
            "IWatch", Map.of("watch-add", 3, "watch-remove", 2, "watch-list", 1)));
    installNamespaced(context.defineProtocol("INamespaced", Map.of("name", 1, "namespace", 1)));
    installContext(context.defineProtocol("IContext", Map.of("call", -1)));
    installApplicable(
        context.defineProtocol(
            "IApplicable",
            Map.of("apply-in", 3, "apply-default", 1, "transform-in", 3, "transform-out", 4)));
    installPointer(
        context.defineProtocol("IPointer", Map.of("ptr-context", 1, "ptr-keys", 1, "ptr-val", 2)));
    installSpace(
        context.defineProtocol(
            "ISpace",
            Map.of(
                "context-set", 4,
                "context-unset", 2,
                "context-list", 1,
                "context-get", 2,
                "rt-active", 1,
                "rt-get", 2,
                "rt-start", 2,
                "rt-started?", 2,
                "rt-stopped?", 2,
                "rt-stop", 2)));
    installInvokeIn(context.defineProtocol("IInvokeIn", Map.of("invoke-in", -1)));
    installRuntime(context.defineProtocol("IHasRuntime", Map.of("runtime", 1)));
    installExceptionInfo(context.defineProtocol("IExInfo", Map.of("data", 1)));
    installMetadataValue(context.defineProtocol("IMetadata", Map.of("metatype", 1)));
    installPair(context.defineProtocol("IPair", Map.of("key", 1, "value", 1)));
    installComponent(context.defineProtocol("IComponent", componentMethods()));
    installComponentQuery(context.defineProtocol("IComponentQuery", Map.of(
        "started?", 1, "stopped?", 1, "info", 2, "remote?", 1, "health", 1)));
    installComponentProps(context.defineProtocol("IComponentProps", Map.of("props", 1)));
    installComponentOptions(context.defineProtocol("IComponentOptions", Map.of("options", 1)));
    installComponentTrack(context.defineProtocol("IComponentTrack", Map.of("track-path", 1)));
    installContextLifeCycle(
        context.defineProtocol(
            "IContextLifeCycle",
            Map.of(
                "has-module?", 2,
                "setup-module", 2,
                "teardown-module", 2,
                "has-pointer?", 2,
                "setup-pointer", 2,
                "teardown-pointer", 2)));
    installHashCached(
        context.defineProtocol(
            "IHashCached", Map.of("hash-current", 1, "hash-put", 2)));
    context.defineProtocol("IMutable", Map.of());
    context.defineProtocol("IPersistent", Map.of());
    context.defineProtocol("IOFn", Map.of());
    installIter(context.defineProtocol("IIter", Map.of("iter", 1)));
    installIterator(
        context.defineProtocol(
            "IIterator", Map.of("iter-next?", 1, "iter-next", 1)));
    installClose(context.defineProtocol("IClose", Map.of("close", 1)));
    installCas(context.defineProtocol("ICas", Map.of("cas", 3)));
    installReduce(context, context.defineProtocol("IReduce", Map.of("reduce", -1)));
    installPromise(
        context.defineProtocol(
            "IPromise",
            Map.of(
                "state", 1,
                "value", 1,
                "then", 2,
                "catch", 2,
                "finally", 2,
                "cancel", 1)));
    installCoroutine(
        context.defineProtocol("ICoroutine", Map.of("status", 1, "resume", -1)));
  }

  public static void installIFn(HaraProtocol protocol) {
    protocol.extend(IFn.class, "invoke", HaraJavaAdapters::invokeFunction);
  }

  public static void installApplicable(HaraProtocol protocol) {
    protocol.extend(
        IApplicable.class,
        "apply-in",
        (receiver, arguments) ->
            ((IApplicable) receiver).applyIn(arguments[0], (Object[]) arguments[1]));
    protocol.extend(
        IApplicable.class,
        "apply-default",
        (receiver, arguments) -> ((IApplicable) receiver).applyDefault());
    protocol.extend(
        IApplicable.class,
        "transform-in",
        (receiver, arguments) ->
            ((IApplicable) receiver).transformIn(arguments[0], (Object[]) arguments[1]));
    protocol.extend(
        IApplicable.class,
        "transform-out",
        (receiver, arguments) ->
            ((IApplicable) receiver)
                .transformOut(arguments[0], (Object[]) arguments[1], arguments[2]));
  }

  public static void installPointer(HaraProtocol protocol) {
    protocol.extend(
        IPointer.class, "ptr-context", (receiver, arguments) -> ((IPointer) receiver).ptrContext());
    protocol.extend(
        IPointer.class, "ptr-keys", (receiver, arguments) -> ((IPointer) receiver).ptrKeys());
    protocol.extend(
        IPointer.class,
        "ptr-val",
        (receiver, arguments) -> ((IPointer) receiver).ptrVal(arguments[0]));
  }

  public static void installSpace(HaraProtocol protocol) {
    protocol.extend(
        ISpace.class,
        "context-set",
        (receiver, arguments) -> {
          ((ISpace) receiver).contextSet(arguments[0], arguments[1], arguments[2]);
          return receiver;
        });
    protocol.extend(
        ISpace.class,
        "context-unset",
        (receiver, arguments) -> {
          ((ISpace) receiver).contextUnset(arguments[0]);
          return receiver;
        });
    protocol.extend(
        ISpace.class, "context-list", (receiver, arguments) -> ((ISpace) receiver).contextList());
    protocol.extend(
        ISpace.class,
        "context-get",
        (receiver, arguments) -> ((ISpace) receiver).contextGet(arguments[0]));
    protocol.extend(
        ISpace.class, "rt-active", (receiver, arguments) -> ((ISpace) receiver).activeRuntimes());
    protocol.extend(
        ISpace.class,
        "rt-get",
        (receiver, arguments) -> ((ISpace) receiver).runtimeGet(arguments[0]));
    protocol.extend(
        ISpace.class,
        "rt-start",
        (receiver, arguments) -> ((ISpace) receiver).runtimeStart(arguments[0]));
    protocol.extend(
        ISpace.class,
        "rt-started?",
        (receiver, arguments) -> ((ISpace) receiver).runtimeStarted(arguments[0]));
    protocol.extend(
        ISpace.class,
        "rt-stopped?",
        (receiver, arguments) -> ((ISpace) receiver).runtimeStopped(arguments[0]));
    protocol.extend(
        ISpace.class,
        "rt-stop",
        (receiver, arguments) -> {
          ((ISpace) receiver).runtimeStop(arguments[0]);
          return receiver;
        });
  }

  /** Invokes an existing Java IFn using the same collection lookup semantics as protocol calls. */
  public static Object invokeFunction(Object receiver, Object[] arguments) {
    IFn<?, ?, ?> function = (IFn<?, ?, ?>) receiver;
    if (function instanceof ILookup) {
      return lookupValue((ILookup<?, ?>) function, arguments);
    }
    if (function instanceof ISequentialLookupType && arguments.length == 1) {
      return ((ISequentialLookupType<?>) function).nth(((Number) arguments[0]).longValue());
    }
    if (function instanceof ISetType) {
      return setValue((ISetType<?>) function, arguments);
    }
    return applyFunction(function, arguments);
  }

  public static void installLookup(HaraProtocol protocol) {
    protocol.extend(
        ILookup.class,
        "lookup",
        (receiver, arguments) -> {
          if (arguments.length < 1 || arguments.length > 2) {
            throw new HaraException("ILookup/lookup expects one or two arguments");
          }
          return lookupValue((ILookup<?, ?>) receiver, arguments);
        });
    protocol.extend(byte[].class, "lookup", HaraJavaAdapters::lookupBytes);
    protocol.extendNil(
        "lookup",
        (receiver, arguments) -> {
          if (arguments.length < 1 || arguments.length > 2) {
            throw new HaraException("ILookup/lookup expects one or two arguments");
          }
          return arguments.length == 2 ? arguments[1] : null;
        });
  }

  public static void installAssoc(HaraProtocol protocol) {
    protocol.extend(
        IAssoc.class,
        "assoc",
        (receiver, arguments) -> {
          return assocValue((IAssoc<?, ?>) receiver, arguments);
        });
  }

  public static void installCount(HaraProtocol protocol) {
    protocol.extend(ICount.class, "count", (receiver, arguments) -> ((ICount) receiver).count());
    protocol.extend(byte[].class, "count", (receiver, arguments) -> ((byte[]) receiver).length);
    protocol.extendNil("count", (receiver, arguments) -> 0L);
  }

  public static void installConj(HaraProtocol protocol) {
    protocol.extend(
        IConj.class, "conj", (receiver, arguments) -> conjValue((IConj<?>) receiver, arguments[0]));
    protocol.extendNil("conj", (receiver, arguments) -> List.Standard.from(null, arguments[0]));
  }

  public static void installFind(HaraProtocol protocol) {
    protocol.extend(
        IFind.class,
        "find",
        (receiver, arguments) -> findValue((IFind<?, ?>) receiver, arguments[0]));
  }

  public static void installEquality(HaraProtocol protocol) {
    protocol.extend(
        IEquality.class,
        "equality",
        (receiver, arguments) -> ((IEquality) receiver).equality(arguments[0]));
    protocol.extend(
        byte[].class,
        "equality",
        (receiver, arguments) ->
            arguments.length == 1
                && arguments[0] instanceof byte[]
                && Arrays.equals((byte[]) receiver, (byte[]) arguments[0]));
  }

  public static void installHash(HaraProtocol protocol) {
    protocol.extend(IHash.class, "hash", (receiver, arguments) -> ((IHash) receiver).hashGet());
    protocol.extend(
        byte[].class, "hash", (receiver, arguments) -> (long) Arrays.hashCode((byte[]) receiver));
  }

  public static void installMetadata(HaraProtocol protocol) {
    protocol.extend(IObjType.class, "meta", (receiver, arguments) -> ((IObjType) receiver).meta());
    protocol.extend(
        IObjType.class,
        "with-meta",
        (receiver, arguments) ->
            ((IObjType) receiver).withMeta((hara.lang.protocol.IMetadata) arguments[0]));
  }

  public static void installDeref(HaraProtocol protocol) {
    protocol.extend(IDeref.class, "deref", (receiver, arguments) -> ((IDeref<?>) receiver).deref());
  }

  public static void installDerefTimeout(HaraProtocol protocol) {
    protocol.extend(
        IDerefTimeout.class,
        "deref-timeout",
        (receiver, arguments) ->
            derefTimeoutValue((IDerefTimeout<?>) receiver, arguments[0], arguments[1]));
  }

  public static void installNth(HaraProtocol protocol) {
    protocol.extend(
        INth.class,
        "nth",
        (receiver, arguments) -> ((INth<?>) receiver).nth(((Number) arguments[0]).longValue()));
    protocol.extend(
        byte[].class,
        "nth",
        (receiver, arguments) -> {
          long index = ((Number) arguments[0]).longValue();
          byte[] bytes = (byte[]) receiver;
          if (index < 0 || index >= bytes.length) {
            throw new HaraException("byte index out of bounds: " + index);
          }
          return bytes[(int) index];
        });
  }

  public static void installEmpty(HaraProtocol protocol) {
    protocol.extend(IEmpty.class, "empty", (receiver, arguments) -> ((IEmpty) receiver).empty());
    protocol.extendNil("empty", (receiver, arguments) -> null);
  }

  public static void installDisplay(HaraProtocol protocol) {
    protocol.extend(
        IDisplay.class, "display", (receiver, arguments) -> ((IDisplay) receiver).display());
  }

  public static void installCollection(HaraProtocol protocol) {
    protocol.extend(
        IColl.class, "start-string", (receiver, arguments) -> ((IColl<?>) receiver).startString());
    protocol.extend(
        IColl.class, "end-string", (receiver, arguments) -> ((IColl<?>) receiver).endString());
    protocol.extend(
        IColl.class, "sep-string", (receiver, arguments) -> ((IColl<?>) receiver).sepString());
    protocol.extend(
        IColl.class, "iterator", (receiver, arguments) -> ((IColl<?>) receiver).iterator());
  }

  public static void installCons(HaraProtocol protocol) {
    protocol.extend(
        ICons.class, "cons", (receiver, arguments) -> consValue((ICons<?>) receiver, arguments[0]));
    protocol.extendNil("cons", (receiver, arguments) -> List.Standard.from(null, arguments[0]));
  }

  public static void installDissoc(HaraProtocol protocol) {
    protocol.extend(
        IDissoc.class,
        "dissoc",
        (receiver, arguments) -> dissocValue((IDissoc<?>) receiver, arguments[0]));
  }

  public static void installIndexed(HaraProtocol protocol) {
    protocol.extend(
        IIndexed.class,
        "index-of",
        (receiver, arguments) -> indexOfValue((IIndexed<?, ?>) receiver, arguments[0]));
  }

  public static void installIndexedKV(HaraProtocol protocol) {
    protocol.extend(
        IIndexedKV.class,
        "index-of-key",
        (receiver, arguments) -> indexOfKeyValue((IIndexedKV<?, ?>) receiver, arguments[0]));
    protocol.extend(
        IIndexedKV.class,
        "index-of-val",
        (receiver, arguments) -> indexOfValValue((IIndexedKV<?, ?>) receiver, arguments[0]));
  }

  public static void installPeekFirst(HaraProtocol protocol) {
    protocol.extend(
        IPeekFirst.class,
        "peek-first",
        (receiver, arguments) -> ((IPeekFirst<?>) receiver).peekFirst());
  }

  public static void installPeekLast(HaraProtocol protocol) {
    protocol.extend(
        IPeekLast.class,
        "peek-last",
        (receiver, arguments) -> ((IPeekLast<?>) receiver).peekLast());
  }

  public static void installPopFirst(HaraProtocol protocol) {
    protocol.extend(
        IPopFirst.class, "pop-first", (receiver, arguments) -> ((IPopFirst) receiver).popFirst());
  }

  public static void installPopLast(HaraProtocol protocol) {
    protocol.extend(
        IPopLast.class, "pop-last", (receiver, arguments) -> ((IPopLast) receiver).popLast());
  }

  public static void installPushFirst(HaraProtocol protocol) {
    protocol.extend(
        IPushFirst.class,
        "push-first",
        (receiver, arguments) -> pushFirstValue((IPushFirst<?>) receiver, arguments[0]));
  }

  public static void installPushLast(HaraProtocol protocol) {
    protocol.extend(
        IPushLast.class,
        "push-last",
        (receiver, arguments) -> pushLastValue((IPushLast<?>) receiver, arguments[0]));
  }

  public static void installRanged(HaraProtocol protocol) {
    protocol.extend(
        IRanged.class, "range-max", (receiver, arguments) -> ((IRanged) receiver).rangeMax());
    protocol.extend(
        IRanged.class, "range-min", (receiver, arguments) -> ((IRanged) receiver).rangeMin());
  }

  public static void installContextLifeCycle(HaraProtocol protocol) {
    protocol.extend(
        IContextLifeCycle.class,
        "has-module?",
        (receiver, arguments) -> ((IContextLifeCycle) receiver).hasModule(arguments[0]));
    protocol.extend(
        IContextLifeCycle.class,
        "setup-module",
        (receiver, arguments) -> {
          ((IContextLifeCycle) receiver).setupModule(arguments[0]);
          return receiver;
        });
    protocol.extend(
        IContextLifeCycle.class,
        "teardown-module",
        (receiver, arguments) -> {
          ((IContextLifeCycle) receiver).teardownModule(arguments[0]);
          return receiver;
        });
    protocol.extend(
        IContextLifeCycle.class,
        "has-pointer?",
        (receiver, arguments) -> ((IContextLifeCycle) receiver).hasPointer((IPointer) arguments[0]));
    protocol.extend(
        IContextLifeCycle.class,
        "setup-pointer",
        (receiver, arguments) -> {
          ((IContextLifeCycle) receiver).setupPointer((IPointer) arguments[0]);
          return receiver;
        });
    protocol.extend(
        IContextLifeCycle.class,
        "teardown-pointer",
        (receiver, arguments) -> {
          ((IContextLifeCycle) receiver).teardownPointer((IPointer) arguments[0]);
          return receiver;
        });
  }

  public static void installHashCached(HaraProtocol protocol) {
    protocol.extend(
        IHashCached.class,
        "hash-current",
        (receiver, arguments) -> ((IHashCached) receiver).hashCurrent());
    protocol.extend(
        IHashCached.class,
        "hash-put",
        (receiver, arguments) -> {
          ((IHashCached) receiver).hashPut(((Number) arguments[0]).longValue());
          return receiver;
        });
  }

  public static void installIter(HaraProtocol protocol) {
    protocol.extend(
        IIter.class, "iter", (receiver, arguments) -> ((IIter<?>) receiver).iter());
    protocol.extend(
        Iterable.class, "iter", (receiver, arguments) -> ((Iterable<?>) receiver).iterator());
    protocol.extend(
        Iterator.class, "iter", (receiver, arguments) -> receiver);
    protocol.extendDefault("iter", (receiver, arguments) -> hara.lang.base.Iter.iter(receiver));
  }

  public static void installIterator(HaraProtocol protocol) {
    protocol.extend(
        Iterator.class,
        "iter-next?",
        (receiver, arguments) -> ((Iterator<?>) receiver).hasNext());
    protocol.extend(
        Iterator.class,
        "iter-next",
        (receiver, arguments) -> {
          Iterator<?> iterator = (Iterator<?>) receiver;
          if (!iterator.hasNext()) {
            throw new HaraException("iter-next reached the end of the iterator");
          }
          return iterator.next();
        });
  }

  public static void installClose(HaraProtocol protocol) {
    protocol.extend(
        Iterator.class,
        "close",
        (receiver, arguments) -> {
          hara.lang.base.Iter.close((Iterator<?>) receiver);
          return null;
        });
    protocol.extend(
        AutoCloseable.class,
        "close",
        (receiver, arguments) -> {
          try {
            ((AutoCloseable) receiver).close();
            return null;
          } catch (Exception error) {
            throw new HaraException("close failed: " + error.getMessage());
          }
        });
  }

  public static void installCas(HaraProtocol protocol) {
    protocol.extend(
        ICas.class,
        "cas",
        (receiver, arguments) -> {
          Object oldValue = arguments[0];
          Object newValue = arguments[1];
          if (receiver instanceof hara.lang.data.Atom.Swap swap) {
            swap.validate(newValue);
            boolean changed = swap.cas(oldValue, newValue);
            if (changed) swap.notifyWatches(oldValue, newValue);
            return changed;
          }
          return ((ICas<Object>) receiver).cas(oldValue, newValue);
        });
  }

  public static void installReduce(HaraContext context, HaraProtocol protocol) {
    protocol.extend(
        IReduce.class,
        "reduce",
        (receiver, arguments) -> {
          if (arguments.length == 1) {
            return ((IReduce) receiver).reduce(arguments[0]);
          }
          if (arguments.length == 2) {
            return ((IReduce) receiver).reduce(arguments[0], arguments[1]);
          }
          throw new HaraException("IReduce/reduce expects a function and optional initial value");
        });
    protocol.extendDefault(
        "reduce",
        (receiver, arguments) -> {
          if (arguments.length < 1 || arguments.length > 2) {
            throw new HaraException("IReduce/reduce expects a function and optional initial value");
          }
          Iterator<?> iterator = hara.lang.base.Iter.iter(receiver);
          Object accumulator;
          if (arguments.length == 2) {
            accumulator = arguments[1];
          } else {
            if (!iterator.hasNext()) {
              throw new HaraException("IReduce/reduce cannot reduce an empty value without init");
            }
            accumulator = iterator.next();
          }
          while (iterator.hasNext()) {
            accumulator =
                HaraBox.unwrap(
                    context.invokeCallable(
                        arguments[0], new Object[] {accumulator, iterator.next()}));
          }
          return accumulator;
        });
  }

  public static void installPromise(HaraProtocol protocol) {
    protocol.extend(IPromise.class, "state", (receiver, arguments) -> ((IPromise) receiver).state());
    protocol.extend(IPromise.class, "value", (receiver, arguments) -> ((IPromise) receiver).value());
    protocol.extend(
        IPromise.class, "then", (receiver, arguments) -> ((IPromise) receiver).then(arguments[0]));
    protocol.extend(
        IPromise.class,
        "catch",
        (receiver, arguments) -> ((IPromise) receiver).catchError(arguments[0]));
    protocol.extend(
        IPromise.class,
        "finally",
        (receiver, arguments) -> ((IPromise) receiver).finallyDo(arguments[0]));
    protocol.extend(
        IPromise.class, "cancel", (receiver, arguments) -> ((IPromise) receiver).cancel());
  }

  public static void installCoroutine(HaraProtocol protocol) {
    protocol.extend(
        ICoroutine.class, "status", (receiver, arguments) -> ((ICoroutine) receiver).status());
    protocol.extend(
        ICoroutine.class,
        "resume",
        (receiver, arguments) -> ((ICoroutine) receiver).resume(arguments));
  }

  public static void installRealize(HaraProtocol protocol) {
    protocol.extend(
        IRealize.class,
        "realized?",
        (receiver, arguments) -> ((IRealize<?>) receiver).isRealized());
    protocol.extend(
        IRealize.class, "realize", (receiver, arguments) -> ((IRealize<?>) receiver).realize());
  }

  public static void installReset(HaraProtocol protocol) {
    protocol.extend(
        IReset.class,
        "reset",
        (receiver, arguments) -> resetValue((IReset<?>) receiver, arguments[0]));
  }

  public static void installConversion(HaraProtocol mutable, HaraProtocol persistent) {
    mutable.extend(
        IToMutable.class,
        "to-mutable",
        (receiver, arguments) -> ((IToMutable) receiver).toMutable());
    persistent.extend(
        IToPersistent.class,
        "to-persistent",
        (receiver, arguments) -> ((IToPersistent) receiver).toPersistent());
  }

  public static void installValidate(HaraProtocol protocol) {
    protocol.extend(
        IValidate.class,
        "validate",
        (receiver, arguments) -> validateValue((IValidate<?>) receiver, arguments[0]));
    protocol.extend(
        IValidate.class,
        "validator",
        (receiver, arguments) -> ((IValidate<?>) receiver).getValidator());
  }

  public static void installWatch(HaraProtocol protocol) {
    protocol.extend(
        IWatch.class,
        "watch-add",
        (receiver, arguments) -> {
          IWatch watch = (IWatch) receiver;
          Object callback = arguments[1];
          watch.addWatch(
              arguments[0],
              entry ->
                  invokeCallback(
                      callback,
                      new Object[] {arguments[0], receiver, ((IWatch.WatchEntry) entry).oldVal(),
                          ((IWatch.WatchEntry) entry).newVal()}));
          return receiver;
        });
    protocol.extend(
        IWatch.class,
        "watch-remove",
        (receiver, arguments) -> {
          ((IWatch) receiver).removeWatch(arguments[0]);
          return receiver;
        });
    protocol.extend(
        IWatch.class, "watch-list", (receiver, arguments) -> ((IWatch) receiver).getWatches());
  }

  public static void installNamespaced(HaraProtocol protocol) {
    protocol.extend(
        INamespaced.class, "name", (receiver, arguments) -> ((INamespaced) receiver).getName());
    protocol.extend(
        INamespaced.class,
        "namespace",
        (receiver, arguments) -> ((INamespaced) receiver).getNamespace());
  }

  public static void installContext(HaraProtocol protocol) {
    protocol.extend(
        IContext.class, "call", (receiver, arguments) -> ((IContext) receiver).call(arguments));
  }

  public static void installInvokeIn(HaraProtocol protocol) {
    protocol.extend(
        IInvokeIn.class,
        "invoke-in",
        (receiver, arguments) -> {
          if (arguments.length < 1 || !(arguments[0] instanceof IContext)) {
            throw new HaraException("IInvokeIn/invoke-in expects a context");
          }
          return ((IInvokeIn) receiver)
              .invokeIn(
                  (IContext) arguments[0], Arrays.copyOfRange(arguments, 1, arguments.length));
        });
  }

  public static void installRuntime(HaraProtocol protocol) {
    protocol.extend(
        IHasRuntime.class,
        "runtime",
        (receiver, arguments) -> ((IHasRuntime) receiver).getRuntime());
  }

  public static void installExceptionInfo(HaraProtocol protocol) {
    protocol.extend(IExInfo.class, "data", (receiver, arguments) -> ((IExInfo) receiver).getData());
  }

  public static void installMetadataValue(HaraProtocol protocol) {
    protocol.extend(
        IMetadata.class, "metatype", (receiver, arguments) -> ((IMetadata) receiver).getMetatype());
  }

  public static void installPair(HaraProtocol protocol) {
    protocol.extend(
        IPair.class, "key", (receiver, arguments) -> ((Map.Entry<?, ?>) receiver).getKey());
    protocol.extend(
        IPair.class, "value", (receiver, arguments) -> ((Map.Entry<?, ?>) receiver).getValue());
  }

  public static void installComponent(HaraProtocol protocol) {
    protocol.extend(
        IComponent.class, "props", (receiver, arguments) -> ((IComponent) receiver).getProps());
    protocol.extend(
        IComponent.class, "status", (receiver, arguments) -> ((IComponent) receiver).getStatus());
    protocol.extend(
        IComponent.class, "started?", (receiver, arguments) -> ((IComponent) receiver).isStarted());
    protocol.extend(
        IComponent.class, "stopped?", (receiver, arguments) -> ((IComponent) receiver).isStopped());
    protocol.extend(
        IComponent.class, "start", (receiver, arguments) -> ((IComponent) receiver).start());
    protocol.extend(
        IComponent.class, "stop", (receiver, arguments) -> ((IComponent) receiver).stop());
    protocol.extend(
        IComponent.class, "kill", (receiver, arguments) -> ((IComponent) receiver).kill());
    protocol.extend(
        IComponent.class, "remote?", (receiver, arguments) -> ((IComponent) receiver).isRemote());
  }

  public static void installComponentQuery(HaraProtocol protocol) {
    protocol.extend(IComponentQuery.class, "started?", (receiver, arguments) -> ((IComponentQuery) receiver).started());
    protocol.extend(IComponentQuery.class, "stopped?", (receiver, arguments) -> ((IComponentQuery) receiver).stopped());
    protocol.extend(IComponentQuery.class, "info", (receiver, arguments) -> ((IComponentQuery) receiver).info(arguments[0]));
    protocol.extend(IComponentQuery.class, "remote?", (receiver, arguments) -> ((IComponentQuery) receiver).remote());
    protocol.extend(IComponentQuery.class, "health", (receiver, arguments) -> ((IComponentQuery) receiver).health());
  }

  public static void installComponentProps(HaraProtocol protocol) {
    protocol.extend(IComponentProps.class, "props", (receiver, arguments) -> ((IComponentProps) receiver).props());
  }

  public static void installComponentOptions(HaraProtocol protocol) {
    protocol.extend(
        IComponentOptions.class,
        "options",
        (receiver, arguments) -> ((IComponentOptions) receiver).options());
  }

  public static void installComponentTrack(HaraProtocol protocol) {
    protocol.extend(
        IComponentTrack.class,
        "track-path",
        (receiver, arguments) -> ((IComponentTrack) receiver).trackPath());
  }

  private static Map<String, Integer> metadataMethods() {
    Map<String, Integer> methods = new LinkedHashMap<>();
    methods.put("meta", 1);
    methods.put("with-meta", 2);
    return methods;
  }

  private static Map<String, Integer> collectionMethods() {
    Map<String, Integer> methods = new LinkedHashMap<>();
    methods.put("start-string", 1);
    methods.put("end-string", 1);
    methods.put("sep-string", 1);
    methods.put("iterator", 1);
    return methods;
  }

  private static Map<String, Integer> navigationMethods() {
    Map<String, Integer> methods = new LinkedHashMap<>();
    methods.put("peek-first", 1);
    methods.put("peek-last", 1);
    methods.put("pop-first", 1);
    methods.put("pop-last", 1);
    methods.put("push-first", 2);
    methods.put("push-last", 2);
    return methods;
  }

  private static Map<String, Integer> componentMethods() {
    Map<String, Integer> methods = new LinkedHashMap<>();
    methods.put("props", 1);
    methods.put("status", 1);
    methods.put("started?", 1);
    methods.put("stopped?", 1);
    methods.put("start", 1);
    methods.put("stop", 1);
    methods.put("kill", 1);
    methods.put("remote?", 1);
    return methods;
  }

  private static Object lookupValue(ILookup<?, ?> lookup, Object[] arguments) {
    if (arguments.length < 1 || arguments.length > 2) {
      throw new HaraException("ILookup/lookup expects one or two arguments");
    }
    return lookupValueUnchecked(lookup, arguments);
  }

  private static Object lookupBytes(Object receiver, Object[] arguments) {
    if (arguments.length < 1 || arguments.length > 2 || !(arguments[0] instanceof Number)) {
      throw new HaraException("ILookup/lookup on bytes expects an index and optional default");
    }
    long index = ((Number) arguments[0]).longValue();
    byte[] bytes = (byte[]) receiver;
    if (index < 0 || index >= bytes.length) {
      return arguments.length == 2 ? arguments[1] : null;
    }
    return bytes[(int) index];
  }

  @SuppressWarnings("unchecked")
  private static Object lookupValueUnchecked(ILookup<?, ?> lookup, Object[] arguments) {
    ILookup<Object, Object> typed = (ILookup<Object, Object>) lookup;
    return arguments.length == 1
        ? typed.lookup(arguments[0])
        : typed.lookup(arguments[0], arguments[1]);
  }

  @SuppressWarnings("unchecked")
  private static Object assocValue(IAssoc<?, ?> assoc, Object[] arguments) {
    return ((IAssoc<Object, Object>) assoc).assoc(arguments[0], arguments[1]);
  }

  @SuppressWarnings("unchecked")
  private static Object conjValue(IConj<?> conj, Object value) {
    return ((IConj<Object>) conj).conj(value);
  }

  @SuppressWarnings("unchecked")
  private static Object findValue(IFind<?, ?> find, Object key) {
    return ((IFind<Object, Object>) find).find(key);
  }

  private static Object setValue(ISetType<?> set, Object[] arguments) {
    if (arguments.length < 1 || arguments.length > 2) {
      throw new HaraException("IFn set lookup expects one or two arguments");
    }
    Object found = findValue(set, arguments[0]);
    return found == null && arguments.length == 2 ? arguments[1] : found;
  }

  private static Object invokeCallback(Object callback, Object[] arguments) {
    if (callback instanceof HaraFunction) {
      HaraFunction function = (HaraFunction) callback;
      return function.callTarget().call(function.callArguments(arguments));
    }
    if (callback instanceof IFn) {
      return applyFunction((IFn<?, ?, ?>) callback, arguments);
    }
    if (callback instanceof Consumer<?>) {
      @SuppressWarnings("unchecked")
      Consumer<Object> consumer = (Consumer<Object>) callback;
      consumer.accept(arguments[0]);
      return null;
    }
    throw new HaraException("watch callback must be a Hara function or IFn");
  }

  @SuppressWarnings("unchecked")
  private static Object indexOfValue(IIndexed<?, ?> indexed, Object value) {
    return ((IIndexed<Object, Object>) indexed).indexOf(value);
  }

  @SuppressWarnings("unchecked")
  private static long indexOfKeyValue(IIndexedKV<?, ?> indexed, Object value) {
    return ((IIndexedKV<Object, Object>) indexed).indexOfKey(value);
  }

  @SuppressWarnings("unchecked")
  private static long indexOfValValue(IIndexedKV<?, ?> indexed, Object value) {
    return ((IIndexedKV<Object, Object>) indexed).indexOfVal(value);
  }

  @SuppressWarnings("unchecked")
  private static Object consValue(ICons<?> cons, Object value) {
    return ((ICons<Object>) cons).cons(value);
  }

  @SuppressWarnings("unchecked")
  private static Object dissocValue(IDissoc<?> dissoc, Object key) {
    return ((IDissoc<Object>) dissoc).dissoc(key);
  }

  @SuppressWarnings("unchecked")
  private static Object pushFirstValue(IPushFirst<?> pushFirst, Object value) {
    return ((IPushFirst<Object>) pushFirst).pushFirst(value);
  }

  @SuppressWarnings("unchecked")
  private static Object pushLastValue(IPushLast<?> pushLast, Object value) {
    return ((IPushLast<Object>) pushLast).pushLast(value);
  }

  @SuppressWarnings("unchecked")
  private static Object resetValue(IReset<?> reset, Object value) {
    return ((IReset<Object>) reset).reset(value);
  }

  @SuppressWarnings("unchecked")
  private static Object validateValue(IValidate<?> validate, Object value) {
    return ((IValidate<Object>) validate).validate(value);
  }

  @SuppressWarnings("unchecked")
  private static Object derefTimeoutValue(
      IDerefTimeout<?> deref, Object milliseconds, Object timeoutValue) {
    return ((IDerefTimeout<Object>) deref)
        .derefTimeout(((Number) milliseconds).longValue(), timeoutValue);
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private static Object applyFunction(IFn<?, ?, ?> function, Object[] arguments) {
    return IFn.applyAsArray((IFn) function, arguments);
  }
}
