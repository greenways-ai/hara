package hara.truffle;

import com.oracle.truffle.api.CompilerDirectives.TruffleBoundary;
import com.oracle.truffle.api.TruffleLanguage;
import com.oracle.truffle.api.TruffleFile;
import com.oracle.truffle.api.source.Source;
import hara.kernel.builtin.BuiltinStruct;
import hara.kernel.flavor.NativeCapability;
import hara.kernel.flavor.NativeFlavorAccess;
import hara.kernel.flavor.NativeFlavorException;
import hara.kernel.flavor.NativeFlavorProvider;
import hara.kernel.flavor.NativeFlavorRegistry;
import hara.kernel.jvm.JvmFlavorProvider;
import hara.lang.base.Iter;
import hara.lang.base.Eq;
import hara.lang.base.primitive.Num;
import hara.lang.base.iter.CloseableIterator;
import hara.lang.data.Symbol;
import hara.lang.data.List;
import hara.lang.data.Keyword;
import hara.lang.data.types.IMapType;
import hara.lang.data.types.ILinearType;
import hara.lang.protocol.IFn;
import hara.lang.protocol.IMetadata;
import hara.lang.protocol.IDeref;
import hara.lang.protocol.IDerefTimeout;
import hara.lang.protocol.IDisplay;
import hara.lang.protocol.ICount;
import hara.lang.protocol.INth;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.LinkedHashMap;
import java.util.IdentityHashMap;
import java.util.Collections;
import java.util.Map;
import java.util.Set;
import java.util.Deque;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Iterator;
import java.util.NoSuchElementException;
import java.util.LinkedHashSet;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionException;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicLong;
import java.util.function.Function;
import java.util.function.Supplier;

public final class HaraContext {
  private static final Set<String> SPECIAL_SYMBOLS =
      Set.of(
          "quote",
          "do",
          "if",
          "when",
          "when-not",
          "cond",
          "and",
          "or",
          "let",
          "letfn",
          "binding",
          "loop",
          "recur",
          "throw",
          "try",
          "fn",
          "defn",
          "defn-",
          "declare",
          "defmulti",
          "defmethod",
          "def",
          "var",
          "deref",
          "set!",
          "defstruct",
          "defprotocol",
          "extend-type",
          "defmacro",
          "new",
          "ns");
  private static final String INTRINSIC_NAMESPACE = "hara.lang.intrinsic";
  private static final String FOUNDATION_NAMESPACE = "std.foundation";
  private static final Map<String, String> GENERATED_LIBRARIES =
      Map.of(
          "string", "std.foundation.string",
          "promise", "std.foundation.promise",
          "bytes", "std.foundation.bytes",
          "file", "std.foundation.file",
          "socket", "std.foundation.socket");
  private static final Map<String, String> DEFAULT_LIBRARY_ALIASES =
      Map.of(
          "string", "str",
          "promise", "promise",
          "bytes", "bytes",
          "file", "file",
          "socket", "socket");
  private static final Set<String> MARKER_METHOD_NAMES =
      Set.of(
          "get",
          "set",
          "push-first",
          "push-last",
          "pop-first",
          "pop-last",
          "insert",
          "remove",
          "clone",
          "slice",
          "map",
          "filter",
          "fold-left",
          "fold-right",
          "has?",
          "delete",
          "assign",
          "keys",
          "vals",
          "pairs");
  private final TruffleLanguage.Env environment;
  private final Map<String, HaraNamespace> namespaces = new ConcurrentHashMap<>();
  private final Map<String, Map<String, HaraMacro>> macros = new ConcurrentHashMap<>();
  private final Map<String, Map<String, String>> aliases = new ConcurrentHashMap<>();
  private final Map<String, String> nativeFlavors = new ConcurrentHashMap<>();
  private final Map<String, Map<String, Object>> nativeImports = new ConcurrentHashMap<>();
  private final NativeFlavorRegistry nativeFlavorRegistry =
      new NativeFlavorRegistry().register(JvmFlavorProvider.INSTANCE);
  private final HaraExtensionRegistry extensionRegistry =
      new HaraExtensionRegistry(HaraContext.class.getClassLoader());
  private final HaraLibraryLoader libraryLoader = new HaraLibraryLoader();
  private HaraVar.Origin definitionOrigin = HaraVar.Origin.SOURCE;
  private boolean eagerFallbacksLoading;
  private boolean eagerFallbacksLoaded;
  private volatile HaraProject project;
  private volatile boolean projectDiscovered;
  private final Map<String, HaraExtensionRuntime> loadedExtensions = new ConcurrentHashMap<>();
  private final Map<String, ModuleRecord> modules = new ConcurrentHashMap<>();
  private final Map<String, Set<String>> moduleDependencies = new ConcurrentHashMap<>();
  private final Map<String, Object> libraryStates = new ConcurrentHashMap<>();
  private final Set<String> loadingModules = ConcurrentHashMap.newKeySet();
  private final Deque<String> loadingStack = new ArrayDeque<>();
  private volatile HaraNamespace currentNamespace;
  private final Map<String, Map<String, BuiltinExport>> builtinCatalogs = new ConcurrentHashMap<>();
  private boolean collectingBuiltins;
  private String collectingBuiltinNamespace;
  private final HaraProtocol ifnProtocol;
  private final AtomicLong gensymCounter = new AtomicLong();
  private final Map<Object, LinkedHashMap<Object, Object>> guestWatches =
      Collections.synchronizedMap(new IdentityHashMap<>());

  HaraContext(TruffleLanguage.Env environment) {
    this.environment = environment;
    currentNamespace = namespace(INTRINSIC_NAMESPACE);
    Map<String, Integer> ifnMethods = new LinkedHashMap<>();
    ifnMethods.put("invoke", -1);
    ifnProtocol = new HaraProtocol("IFn", ifnMethods);
    currentNamespace.define(
        "IFn", ifnProtocol, null, HaraVar.Origin.RUNTIME_PRIMITIVE);
    withDefinitionOrigin(
        HaraVar.Origin.RUNTIME_PRIMITIVE,
        () -> {
          HaraJavaAdapters.install(this);
          collectBuiltins(
              FOUNDATION_NAMESPACE,
              () -> {
                installNumericBuiltins(namespace(FOUNDATION_NAMESPACE));
                installCoreBuiltins(namespace(FOUNDATION_NAMESPACE));
              });
        });
    installProjectMacro();
    installNativeLibraries();
    collectBuiltins(FOUNDATION_NAMESPACE, () -> libraryLoader.installEagerJava(this));
    currentNamespace = namespace("user");
    initializeUserNamespace(currentNamespace);
  }

  TruffleLanguage.Env environment() {
    return environment;
  }

  void closeExtensions() {
    for (HaraExtensionRuntime extension : loadedExtensions.values()) {
      extension.close();
    }
    loadedExtensions.clear();
  }

  private HaraNamespace namespace(String name) {
    return namespaces.computeIfAbsent(name, HaraNamespace::new);
  }

  private void withDefinitionOrigin(HaraVar.Origin origin, Runnable action) {
    HaraVar.Origin previous = definitionOrigin;
    definitionOrigin = origin;
    try {
      action.run();
    } finally {
      definitionOrigin = previous;
    }
  }

  synchronized void ensureEagerFallbacks() {
    if (eagerFallbacksLoaded || eagerFallbacksLoading) return;
    eagerFallbacksLoading = true;
    try {
      libraryLoader.installEagerFallbacks(this);
      eagerFallbacksLoaded = true;
    } finally {
      eagerFallbacksLoading = false;
    }
  }

  void collectBuiltins(String namespaceName, Runnable definitions) {
    boolean previousCollecting = collectingBuiltins;
    String previousNamespace = collectingBuiltinNamespace;
    collectingBuiltins = true;
    collectingBuiltinNamespace = namespaceName;
    try {
      definitions.run();
    } finally {
      collectingBuiltins = previousCollecting;
      collectingBuiltinNamespace = previousNamespace;
    }
  }

  private void registerBuiltin(
      String namespaceName,
      String symbolName,
      Object value,
      IMetadata metadata,
      HaraVar.Origin origin) {
    Map<String, BuiltinExport> catalog =
        builtinCatalogs.computeIfAbsent(namespaceName, ignored -> new LinkedHashMap<>());
    BuiltinExport previous =
        catalog.putIfAbsent(
            symbolName, new BuiltinExport(namespaceName, symbolName, value, metadata, origin));
    if (previous != null) {
      throw new HaraException("Duplicate builtin export: " + namespaceName + "/" + symbolName);
    }
  }

  private void activateBuiltins(HaraNamespaceDeclaration declaration) {
    if (declaration.builtins.isEmpty()) return;
    Map<String, BuiltinExport> catalog = builtinCatalogs.get(declaration.name.getName());
    if (catalog == null) {
      throw new HaraException(
          "No host builtins are registered for namespace: " + declaration.name.getName());
    }
    HaraNamespace target = namespace(declaration.name.getName());
    for (Symbol requested : declaration.builtins) {
      BuiltinExport export = catalog.get(requested.getName());
      if (export == null) {
        throw new HaraException(
            "Missing builtin export: " + declaration.name.getName() + "/" + requested.getName());
      }
      HaraVar existing = target.lookup(requested.getName());
      if (existing != null) {
        if (declaration.name.getName().equals(existing.namespaceName())
            && existing.get() == export.value
            && existing.origin() == export.origin) {
          continue;
        }
        throw new HaraException(
            "Builtin activation conflicts with existing binding: "
                + declaration.name.getName()
                + "/"
                + requested.getName());
      }
      target.define(
          requested.getName(), export.value, export.metadata, export.origin);
      if (export.value instanceof HaraMacro macro) {
        macros
            .computeIfAbsent(declaration.name.getName(), ignored -> new ConcurrentHashMap<>())
            .put(requested.getName(), macro);
      }
    }
  }

  void defineLibraryFunction(
      String namespaceName,
      String symbolName,
      Function<Object[], Object> implementation,
      IMetadata metadata) {
    namespace(namespaceName)
        .define(
            symbolName,
            new VariadicBuiltin(namespaceName + "/" + symbolName, implementation),
            metadata,
            HaraVar.Origin.JAVA_LIBRARY);
  }

  void defineLibraryValue(
      String namespaceName, String symbolName, Object value, IMetadata metadata) {
    namespace(namespaceName)
        .define(symbolName, value, metadata, HaraVar.Origin.JAVA_LIBRARY);
  }

  void defineLibraryMacro(
      String namespaceName,
      String symbolName,
      Function<List<?>, Object> expander,
      IMetadata metadata,
      boolean intrinsic) {
    HaraMacro macro = HaraMacro.nativeMacro(Symbol.create(symbolName), expander);
    namespace(namespaceName)
        .define(symbolName, macro, metadata, HaraVar.Origin.JAVA_LIBRARY);
    macros
        .computeIfAbsent(namespaceName, ignored -> new ConcurrentHashMap<>())
        .put(symbolName, macro);
    if (intrinsic) defineIntrinsicMacro(Symbol.create(symbolName), macro);
  }

  Object libraryFunction(String name, Function<Object[], Object> implementation) {
    return new VariadicBuiltin(name, implementation);
  }

  String currentNamespaceName() {
    return currentNamespace.name();
  }

  public Symbol gensym(String prefix) {
    String base = prefix == null || prefix.isEmpty() ? "G__" : prefix + "__";
    return Symbol.create(base + gensymCounter.incrementAndGet());
  }

  Object macroAliases() {
    ArrayList<Object> entries = new ArrayList<>();
    for (Map.Entry<String, String> entry :
        aliases.getOrDefault(currentNamespace.name(), Map.of()).entrySet()) {
      entries.add(Symbol.create(entry.getKey()));
      entries.add(Symbol.create(entry.getValue()));
    }
    return hara.lang.data.Map.Standard.from(null, entries.toArray());
  }

  void runInNamespace(String namespaceName, Runnable operation) {
    HaraNamespace previous = currentNamespace;
    try {
      currentNamespace = namespace(namespaceName);
      operation.run();
    } finally {
      currentNamespace = previous;
    }
  }

  @SuppressWarnings("unchecked")
  <T> T libraryState(String name, Supplier<T> factory) {
    return (T) libraryStates.computeIfAbsent(name, ignored -> factory.get());
  }

  @TruffleBoundary
  public void setCurrentNamespace(Symbol symbol) {
    setCurrentNamespace(symbol, new Object[0]);
  }

  @TruffleBoundary
  public void setCurrentNamespace(Symbol symbol, Object[] clauses) {
    HaraNamespaceDeclaration declaration = HaraNamespaceDeclaration.parse(symbol, clauses);
    ContextSnapshot snapshot = snapshot();
    try {
      applyNamespaceDeclaration(declaration);
    } catch (RuntimeException error) {
      restore(snapshot);
      throw error;
    }
  }

  private void applyNamespaceDeclaration(HaraNamespaceDeclaration declaration) {
    currentNamespace = namespace(declaration.name.getName());
    referRuntimeIntrinsics(currentNamespace);
    if (!declaration.blank) referFoundation(currentNamespace);
    configureFoundationAliases(declaration);
    activateBuiltins(declaration);
    configureNativeFlavor(declaration.structuralClauses);
    applyNamespaceRequires(declaration.structuralClauses);
  }

  private void configureFoundationAliases(HaraNamespaceDeclaration declaration) {
    Map<String, String> namespaceAliases =
        aliases.computeIfAbsent(currentNamespace.name(), ignored -> new ConcurrentHashMap<>());
    namespaceAliases
        .entrySet()
        .removeIf(entry -> GENERATED_LIBRARIES.containsValue(entry.getValue()));
    for (Map.Entry<String, String> library : GENERATED_LIBRARIES.entrySet()) {
      if (declaration.excludedIntrinsics.contains(library.getKey())) continue;
      String alias =
          declaration.intrinsicAliases.getOrDefault(
              library.getKey(), DEFAULT_LIBRARY_ALIASES.get(library.getKey()));
      putAlias(namespaceAliases, alias, library.getValue());
    }
  }

  private void applyNamespaceRequires(Object[] clauses) {
    Map<String, String> namespaceAliases =
        aliases.computeIfAbsent(currentNamespace.name(), ignored -> new ConcurrentHashMap<>());
    for (Object clauseValue : clauses) {
      List<?> clause = (List<?>) clauseValue;
      if (!(clause.nth(0) instanceof Keyword keyword)
          || !"require".equals(keyword.getName())) {
        continue;
      }
      for (int index = 1; index < clause.count(); index++) {
        applyGeneratedRequire(clause.nth(index), namespaceAliases);
      }
    }
  }

  private void configureNativeFlavor(Object[] clauses) {
    List<?> flavorClause = null;
    for (Object clauseValue : clauses) {
      if (!(clauseValue instanceof List<?>)) continue;
      List<?> clause = (List<?>) clauseValue;
      if (clause.count() > 0
          && clause.nth(0) instanceof Keyword
          && "flavor".equals(((Keyword) clause.nth(0)).getName())) {
        if (flavorClause != null) throw new HaraException("ns accepts only one :flavor clause");
        flavorClause = clause;
      }
    }
    if (flavorClause != null) {
      if (flavorClause.count() != 2 || !(flavorClause.nth(1) instanceof Keyword)) {
        throw new HaraException(":flavor expects one keyword");
      }
      Keyword flavor = (Keyword) flavorClause.nth(1);
      if (flavor.getNamespace() != null) {
        throw new HaraException(":flavor expects an unqualified keyword");
      }
      nativeFlavorRegistry.require(flavor.getName());
      nativeFlavors.put(currentNamespace.name(), flavor.getName());
    }

    for (Object clauseValue : clauses) {
      if (!(clauseValue instanceof List<?>)) continue;
      List<?> clause = (List<?>) clauseValue;
      if (clause.count() == 0
          || !(clause.nth(0) instanceof Keyword)
          || !"import".equals(((Keyword) clause.nth(0)).getName())) continue;
      NativeFlavorProvider provider = nativeProvider();
      if (provider == null) throw new HaraException(":import requires an ns :flavor declaration");
      Map<String, Object> imports =
          nativeImports.computeIfAbsent(
              currentNamespace.name(), ignored -> new ConcurrentHashMap<>());
      for (int i = 1; i < clause.count(); i++) {
        Object spec = clause.nth(i);
        if (spec instanceof Symbol) {
          importNativeType(provider, imports, ((Symbol) spec).display());
        } else if (spec instanceof ILinearType<?>) {
          ILinearType<?> group = (ILinearType<?>) spec;
          if (group.count() == 0) continue;
          Object packageValue = group.nth(0);
          String packageName =
              packageValue instanceof Symbol
                  ? ((Symbol) packageValue).display()
                  : String.valueOf(packageValue);
          for (int j = 1; j < group.count(); j++) {
            Object classValue = group.nth(j);
            String className =
                classValue instanceof Symbol
                    ? ((Symbol) classValue).display()
                    : String.valueOf(classValue);
            importNativeType(provider, imports, packageName + "." + className);
          }
        } else {
          throw new HaraException(":import expects class symbols or package vectors");
        }
      }
    }
  }

  private void importNativeType(
      NativeFlavorProvider provider, Map<String, Object> imports, String name) {
    Object type = provider.resolveType(name, nativeAccess());
    String simpleName = name.substring(name.lastIndexOf('.') + 1);
    Object previous = imports.putIfAbsent(simpleName, type);
    if (previous != null && previous != type) {
      throw new HaraException("Native import already exists: " + simpleName);
    }
  }

  private void initializeUserNamespace(HaraNamespace target) {
    referRuntimeIntrinsics(target);
    referFoundation(target);
    Map<String, String> namespaceAliases =
        aliases.computeIfAbsent(target.name(), ignored -> new ConcurrentHashMap<>());
    for (Map.Entry<String, String> entry : DEFAULT_LIBRARY_ALIASES.entrySet()) {
      namespaceAliases.putIfAbsent(entry.getValue(), GENERATED_LIBRARIES.get(entry.getKey()));
    }
  }

  private void referRuntimeIntrinsics(HaraNamespace target) {
    HaraNamespace intrinsic = namespace(INTRINSIC_NAMESPACE);
    for (Map.Entry<String, HaraVar> entry : intrinsic.vars.entrySet()) {
      if (target.lookup(entry.getKey()) == null) target.refer(entry.getKey(), entry.getValue());
    }
  }

  private void referFoundation(HaraNamespace target) {
    HaraNamespace core = namespace(FOUNDATION_NAMESPACE);
    for (Map.Entry<String, HaraVar> entry : core.vars.entrySet()) {
      if (target.lookup(entry.getKey()) == null) target.refer(entry.getKey(), entry.getValue());
    }
  }

  private void putAlias(Map<String, String> namespaceAliases, String alias, String target) {
    String previous = namespaceAliases.putIfAbsent(alias, target);
    if (previous != null && !previous.equals(target)) {
      throw new HaraException("Namespace alias already refers to " + previous + ": " + alias);
    }
  }

  private void applyGeneratedRequire(Object specValue, Map<String, String> namespaceAliases) {
    if (!(specValue instanceof ILinearType<?>)) {
      throw new HaraException(":require expects vectors such as [std.foundation.string :as str]");
    }
    ILinearType<?> spec = (ILinearType<?>) specValue;
    if (spec.count() == 0 || !(spec.nth(0) instanceof Symbol)) {
      throw new HaraException(":require namespace must be a symbol");
    }
    String target = ((Symbol) spec.nth(0)).display();
    HaraNamespace required = requiredNamespace(target);
    if (required == null) throw new HaraException("Cannot require missing namespace: " + target);
    java.util.Set<String> excludedRefers = new java.util.HashSet<>();
    for (int i = 1; i < spec.count(); i += 2) {
      if (i + 1 >= spec.count() || !(spec.nth(i) instanceof Keyword)) {
        throw new HaraException("Malformed :require options for " + target);
      }
      if ("exclude".equals(((Keyword) spec.nth(i)).getName())) {
        Object excluded = spec.nth(i + 1);
        if (!(excluded instanceof ILinearType<?>)) {
          throw new HaraException(":require :exclude expects a vector of symbols");
        }
        for (Object value : (ILinearType<?>) excluded) {
          if (!(value instanceof Symbol) || ((Symbol) value).getNamespace() != null) {
            throw new HaraException(":require :exclude expects unqualified symbols");
          }
          excludedRefers.add(((Symbol) value).getName());
        }
      }
    }
    for (int i = 1; i < spec.count(); i += 2) {
      if (i + 1 >= spec.count() || !(spec.nth(i) instanceof Keyword)) {
        throw new HaraException("Malformed :require options for " + target);
      }
      String option = ((Keyword) spec.nth(i)).getName();
      Object value = spec.nth(i + 1);
      if ("as".equals(option)) {
        if (!(value instanceof Symbol) || ((Symbol) value).getNamespace() != null) {
          throw new HaraException(":require :as expects an unqualified symbol");
        }
        putAlias(namespaceAliases, ((Symbol) value).getName(), target);
      } else if ("refer".equals(option)) {
        if (value instanceof Keyword && "all".equals(((Keyword) value).getName())) {
          for (String referred : required.symbolNames()) {
            if (!excludedRefers.contains(referred)) currentNamespace.refer(referred, required.lookup(referred));
          }
        } else {
          if (!(value instanceof ILinearType<?>)) {
            throw new HaraException(":require :refer expects a vector of symbols or :all");
          }
          for (Object referred : (ILinearType<?>) value) {
            if (!(referred instanceof Symbol) || ((Symbol) referred).getNamespace() != null) {
              throw new HaraException(":require :refer expects unqualified symbols");
            }
            String name = ((Symbol) referred).getName();
            if (excludedRefers.contains(name)) continue;
            HaraVar variable = required.lookup(name);
            if (variable == null) {
              throw new HaraException("Cannot refer missing var " + name + " from " + target);
            }
            currentNamespace.refer(name, variable);
          }
        }
      } else if ("refer-macros".equals(option)) {
        if (!(value instanceof ILinearType<?>)) {
          throw new HaraException(":require :refer-macros expects a vector of symbols");
        }
        Map<String, HaraMacro> targetMacros = macros.get(target);
        for (Object referred : (ILinearType<?>) value) {
          if (!(referred instanceof Symbol) || ((Symbol) referred).getNamespace() != null) {
            throw new HaraException(":require :refer-macros expects unqualified symbols");
          }
          String name = ((Symbol) referred).getName();
          HaraMacro macro = targetMacros == null ? null : targetMacros.get(name);
          if (macro == null) {
            throw new HaraException("Cannot refer missing macro " + name + " from " + target);
          }
          macros
              .computeIfAbsent(
                  currentNamespace.name(), ignored -> new ConcurrentHashMap<>())
              .put(name, macro);
        }
      } else {
        throw new HaraException("Unsupported :require option: :" + option);
      }
    }
  }

  private synchronized HaraNamespace requiredNamespace(String target) {
    libraryLoader.ensure(this, target);
    HaraNamespace existing = namespaces.get(target);
    if (existing != null) return existing;
    HaraNamespace sourceNamespace = requireSourceNamespace(target);
    if (sourceNamespace != null) return sourceNamespace;
    Path extensionRoot = null;
    if (environment.isFileIOAllowed()) {
      HaraProject currentProject = project();
      extensionRoot = currentProject == null ? null : currentProject.extensionRoot();
    }
    HaraExtensionPackage extensionPackage = extensionRegistry.discover(target, extensionRoot);
    if (extensionPackage == null) return null;
    return installExtension(extensionPackage);
  }

  void loadLibraryFallback(String namespaceName, String resourceName, boolean reload) {
    String previousNamespace = currentNamespace.name();
    HaraVar.Origin previousOrigin = definitionOrigin;
    ContextSnapshot snapshot = snapshot();
    currentNamespace = namespace(namespaceName);
    definitionOrigin = HaraVar.Origin.HAL_FALLBACK;
    try {
      requireModule(
          reload
              ? new Object[] {
                "classpath:" + resourceName,
                hara.lang.data.Map.Standard.from(
                    null, Keyword.create("reload"), Boolean.TRUE)
              }
              : new Object[] {"classpath:" + resourceName});
      ModuleRecord loaded = modules.get("classpath:" + resourceName);
      if (loaded != null && !namespaceName.equals(loaded.namespace)) {
        throw new HaraException(
            "Library fallback "
                + resourceName
                + " loaded namespace "
                + loaded.namespace
                + " instead of "
                + namespaceName);
      }
    } catch (RuntimeException error) {
      restore(snapshot);
      throw error;
    } finally {
      definitionOrigin = previousOrigin;
      currentNamespace = namespace(previousNamespace);
      initializeUserNamespace(currentNamespace);
    }
  }

  private HaraNamespace requireSourceNamespace(String target) {
    return requireSourceNamespace(target, false);
  }

  private HaraNamespace requireSourceNamespace(String target, boolean reload) {
    String resourceName = namespaceResource(target);
    if (environment.isFileIOAllowed()) {
      HaraProject currentProject = project();
      Path source =
          currentProject == null
              ? null
              : currentProject.resolve(target, target.endsWith("-test"));
      if (source != null) {
        requireResolvedSource(source.toString(), reload);
        return loadedSourceNamespace(target, source.toString());
      }
    }
    if (getResource(resourceName) != null) {
      requireResolvedSource("classpath:" + resourceName, reload);
      return loadedSourceNamespace(target, "classpath:" + resourceName);
    }
    return null;
  }

  private void requireResolvedSource(String source, boolean reload) {
    if (!reload) {
      requireModule(new Object[] {source});
      return;
    }
    requireModule(
        new Object[] {
          source,
          hara.lang.data.Map.Standard.from(
              null, Keyword.create("reload"), Boolean.TRUE)
        });
  }

  private HaraNamespace loadedSourceNamespace(String target, String source) {
    HaraNamespace loaded = namespaces.get(target);
    if (loaded == null) {
      throw new HaraException(
          "Namespace source " + source + " did not declare requested namespace " + target);
    }
    return loaded;
  }

  private HaraProject project() {
    if (!projectDiscovered) {
      synchronized (this) {
        if (!projectDiscovered) {
          String workingDirectory =
              environment.getPublicTruffleFile(".").getAbsoluteFile().normalize().getPath();
          project = HaraProject.discover(Path.of(workingDirectory));
          projectDiscovered = true;
        }
      }
    }
    return project;
  }

  private static String namespaceResource(String namespace) {
    return namespace.replace('.', '/').replace('-', '_') + ".hal";
  }

  private HaraNamespace installExtension(HaraExtensionPackage extensionPackage) {
    HaraExtensionManifest manifest = extensionPackage.manifest();
    HaraExtensionRuntime extension =
        "wasm".equals(manifest.provider())
            ? new HaraWasmExtension(extensionPackage)
            : new HaraProcessExtension(
                extensionPackage, environment.isCreateProcessAllowed());
    HaraNamespace generated = namespace(manifest.namespace());
    for (Map.Entry<String, HaraExtensionManifest.Export> export : manifest.exports().entrySet()) {
      String name = export.getKey();
      generated.define(
          name,
          new VariadicBuiltin(
              manifest.namespace() + "/" + name,
              values -> invokeExtension(extension, name, export.getValue(), values)));
    }
    loadedExtensions.put(manifest.namespace(), extension);
    return generated;
  }

  private Object invokeExtension(
      HaraExtensionRuntime extension,
      String name,
      HaraExtensionManifest.Export export,
      Object[] values) {
    if (extension.asynchronous()) return new HaraPromise(extension.invokeAsync(name, values));
    if (!export.async()) return extension.invoke(name, values);
    return new HaraPromise(CompletableFuture.supplyAsync(() -> extension.invoke(name, values)));
  }

  @TruffleBoundary
  public HaraVar resolve(Symbol symbol) {
    String namespaceName = symbol.getNamespace();
    if (namespaceName != null) {
      namespaceName =
          aliases
              .getOrDefault(currentNamespace.name(), Map.of())
              .getOrDefault(namespaceName, namespaceName);
      if (!namespaces.containsKey(namespaceName) && libraryLoader.provides(namespaceName)) {
        libraryLoader.ensure(this, namespaceName);
      }
    }
    HaraNamespace namespace =
        namespaceName == null ? currentNamespace : namespaces.get(namespaceName);
    return namespace == null ? null : namespace.lookup(symbol.getName());
  }

  Symbol canonicalSymbol(Symbol symbol) {
    HaraVar variable = resolve(symbol);
    if (variable != null) {
      return Symbol.create(variable.namespaceName(), variable.symbolName());
    }
    return symbol;
  }

  void declareCurrent(Symbol symbol) {
    HaraVar existing = currentNamespace.lookup(symbol.getName());
    if (existing == null || !currentNamespace.name().equals(existing.namespaceName())) {
      currentNamespace.define(
          symbol.getName(), null, symbol.meta(), definitionOrigin);
    }
  }

  /** Names visible in the current namespace, used by interactive tooling. */
  public java.util.List<String> currentSymbolNames() {
    LinkedHashSet<String> names = new LinkedHashSet<>(currentNamespace.symbolNames());
    names.addAll(MARKER_METHOD_NAMES);
    nativeImports
        .getOrDefault(currentNamespace.name(), Map.of())
        .forEach(
            (simpleName, type) -> {
              names.add(simpleName);
              if (!(type instanceof Class<?>)) return;
              Class<?> cls = (Class<?>) type;
              java.util.Arrays.stream(cls.getFields())
                  .filter(field -> java.lang.reflect.Modifier.isStatic(field.getModifiers()))
                  .forEach(field -> names.add(simpleName + "/" + field.getName()));
              java.util.Arrays.stream(cls.getMethods())
                  .filter(method -> java.lang.reflect.Modifier.isStatic(method.getModifiers()))
                  .forEach(method -> names.add(simpleName + "/" + method.getName()));
            });
    namespaces.forEach(
        (namespaceName, namespace) -> {
          if (!namespaceName.startsWith("hara.native.")) return;
          for (String name : namespace.symbolNames()) names.add(namespaceName + "/" + name);
        });
    for (Map.Entry<String, String> alias :
        aliases.getOrDefault(currentNamespace.name(), Map.of()).entrySet()) {
      if (!namespaces.containsKey(alias.getValue()) && libraryLoader.provides(alias.getValue())) {
        libraryLoader.ensure(this, alias.getValue());
      }
      HaraNamespace target = namespaces.get(alias.getValue());
      if (target == null) continue;
      for (String name : target.symbolNames()) names.add(alias.getKey() + "/" + name);
    }
    return new ArrayList<>(names);
  }

  @TruffleBoundary
  public Object macroExpand(Object form, boolean recursive) {
    Object result = form;
    int expansions = 0;
    do {
      Object expanded = macroExpandOnce(result, runtimeMacroEnvironment(result));
      if (expanded == result) return result;
      result = expanded;
      expansions++;
      if (expansions > 1000) throw new HaraException("macro expansion exceeded 1000 steps");
    } while (recursive);
    return result;
  }

  private Object macroExpandOnce(Object form, Object environment) {
    if (!(form instanceof List<?> list)
        || list.count() == 0
        || !(list.nth(0) instanceof Symbol operator)) {
      return form;
    }
    HaraMacro macro = resolveMacro(operator);
    return macro == null ? form : macro.expand(list, environment);
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private Object runtimeMacroEnvironment(Object form) {
    ArrayList<Object> entries = new ArrayList<>();
    entries.add(Keyword.create("ns"));
    entries.add(Symbol.create(currentNamespace.name()));
    entries.add(Keyword.create("locals"));
    entries.add(hara.lang.data.Map.Standard.EMPTY);
    entries.add(Keyword.create("aliases"));
    entries.add(macroAliases());
    if (form instanceof hara.lang.protocol.IObjType object
        && object.meta() instanceof IMapType<?, ?> metadata) {
      for (String key : new String[] {"file", "line", "column"}) {
        Object value = ((IMapType) metadata).lookup(Keyword.create(key));
        if (value != null) {
          entries.add(Keyword.create(key));
          entries.add(value);
        }
      }
    }
    return hara.lang.data.Map.Standard.from(null, entries.toArray());
  }
  @TruffleBoundary
  public void defineAlias(Symbol alias, Symbol target) {
    if (alias.getNamespace() != null || target.getNamespace() != null) {
      throw new HaraException("alias names must be unqualified");
    }
    if (!namespaces.containsKey(target.getName())) {
      throw new HaraException("Cannot alias missing namespace: " + target.getName());
    }
    aliases
        .computeIfAbsent(currentNamespace.name(), ignored -> new ConcurrentHashMap<>())
        .put(alias.getName(), target.getName());
  }

  @TruffleBoundary
  public HaraVar define(Symbol symbol, Object value) {
    if (symbol.getNamespace() != null && !symbol.getNamespace().equals(currentNamespace.name())) {
      throw new HaraException("Cannot define a var in another namespace: " + symbol.display());
    }
    return currentNamespace.define(symbol.getName(), value, symbol.meta(), definitionOrigin);
  }

  public HaraProtocol ifnProtocol() {
    return ifnProtocol;
  }

  HaraProtocol defineProtocol(String name, Map<String, Integer> methodArities) {
    HaraProtocol protocol = new HaraProtocol(name, methodArities);
    currentNamespace.define(name, protocol, null, definitionOrigin);
    return protocol;
  }

  public boolean hostInteropAllowed() {
    return environment.isHostLookupAllowed();
  }

  public Object asGuestValue(Object value) {
    return environment.asGuestValue(value);
  }

  public boolean isHostObject(Object value) {
    return environment.isHostObject(value);
  }

  public Object asHostObject(Object value) {
    return environment.asHostObject(value);
  }

  public Object lookupHostSymbol(String name) {
    return environment.lookupHostSymbol(name);
  }

  private NativeFlavorAccess nativeAccess() {
    ClassLoader loader = Thread.currentThread().getContextClassLoader();
    if (loader == null) loader = HaraContext.class.getClassLoader();
    Set<NativeCapability> capabilities =
        hostInteropAllowed() ? Set.of(NativeCapability.REFLECTION) : Set.of();
    return NativeFlavorAccess.of(loader, capabilities);
  }

  private NativeFlavorProvider nativeProvider() {
    String flavor = nativeFlavors.get(currentNamespace.name());
    return flavor == null ? null : nativeFlavorRegistry.require(flavor);
  }

  private JvmFlavorProvider jvmProvider() {
    NativeFlavorProvider provider = nativeProvider();
    if (!(provider instanceof JvmFlavorProvider)) {
      throw new HaraException("JVM native operation requires an ns :flavor :jvm declaration");
    }
    return (JvmFlavorProvider) provider;
  }

  public boolean hasNativeSymbol(Symbol symbol) {
    Map<String, Object> imports = nativeImports.get(currentNamespace.name());
    if (imports == null) return false;
    String importedName = symbol.getNamespace() == null ? symbol.getName() : symbol.getNamespace();
    return imports.containsKey(importedName);
  }

  public Object resolveNativeSymbol(Symbol symbol) {
    Map<String, Object> imports = nativeImports.get(currentNamespace.name());
    if (imports == null) throw new HaraException("No native imports in the current namespace");
    String importedName = symbol.getNamespace() == null ? symbol.getName() : symbol.getNamespace();
    Object type = imports.get(importedName);
    if (type == null) throw new HaraException("Native type is not imported: " + importedName);
    if (symbol.getNamespace() == null) return type;
    NativeFlavorProvider provider = nativeProvider();
    try {
      return provider.readStatic(type, symbol.getName(), nativeAccess());
    } catch (NativeFlavorException error) {
      if (error.kind() != NativeFlavorException.Kind.UNSUPPORTED) throw error;
      return new VariadicBuiltin(
          symbol.display(),
          arguments -> provider.invokeStatic(type, symbol.getName(), arguments, nativeAccess()));
    }
  }

  public boolean matchesNativeThrowable(Symbol type, Throwable throwable) {
    if (!hasNativeSymbol(type)) return false;
    NativeFlavorProvider provider = nativeProvider();
    return provider != null
        && provider.matchesThrowable(resolveNativeSymbol(type), throwable, nativeAccess());
  }

  public Object constructNative(Object type, Object[] arguments) {
    NativeFlavorProvider provider = nativeProvider();
    if (provider == null) throw new HaraException("new requires an ns :flavor declaration");
    return provider.construct(HaraBox.unwrap(type), arguments, nativeAccess());
  }

  public Object readNativeMember(Object receiver, String member) {
    NativeFlavorProvider provider = nativeProvider();
    if (provider == null)
      throw new HaraException("Native member access requires an ns :flavor declaration");
    return provider.readMember(HaraBox.unwrap(receiver), member, nativeAccess());
  }

  public Object indexNative(Object receiver, Object index) {
    NativeFlavorProvider provider = nativeProvider();
    if (provider == null)
      throw new HaraException("Native indexed access requires an ns :flavor declaration");
    return provider.index(HaraBox.unwrap(receiver), HaraBox.unwrap(index), nativeAccess());
  }

  @TruffleBoundary
  HaraMacro resolveMacro(Symbol symbol) {
    String namespace = symbol.getNamespace();
    if (namespace != null) {
      namespace = aliases.getOrDefault(currentNamespace.name(), Map.of())
          .getOrDefault(namespace, namespace);
    }
    String namespaceName = namespace == null ? currentNamespace.name() : namespace;
    Map<String, HaraMacro> namespaceMacros = macros.get(namespaceName);
    HaraMacro macro = namespaceMacros == null ? null : namespaceMacros.get(symbol.getName());
    if (macro == null && namespace == null) {
      Map<String, HaraMacro> foundationMacros = macros.get(FOUNDATION_NAMESPACE);
      macro = foundationMacros == null ? null : foundationMacros.get(symbol.getName());
    }
    if (macro != null || INTRINSIC_NAMESPACE.equals(namespaceName)) return macro;
    if (namespace != null) return null;
    Map<String, HaraMacro> intrinsicMacros = macros.get(INTRINSIC_NAMESPACE);
    return intrinsicMacros == null ? null : intrinsicMacros.get(symbol.getName());
  }

  void defineMacro(Symbol symbol, HaraMacro macro) {
    if (symbol.getNamespace() != null) {
      throw new HaraException("defmacro name must not be qualified");
    }
    HaraVar existing = currentNamespace.lookup(symbol.getName());
    if (definitionOrigin == HaraVar.Origin.HAL_FALLBACK
        && existing != null
        && (existing.origin() == HaraVar.Origin.JAVA_LIBRARY
            || existing.origin() == HaraVar.Origin.RUNTIME_PRIMITIVE)) {
      return;
    }
    macros
        .computeIfAbsent(currentNamespace.name(), ignored -> new ConcurrentHashMap<>())
        .put(symbol.getName(), macro);
    currentNamespace.define(symbol.getName(), macro, symbol.meta(), definitionOrigin);
  }

  private void defineIntrinsicMacro(Symbol symbol, HaraMacro macro) {
    HaraNamespace previous = currentNamespace;
    try {
      currentNamespace = namespace(INTRINSIC_NAMESPACE);
      defineMacro(symbol, macro);
    } finally {
      currentNamespace = previous;
    }
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private void installProjectMacro() {
    defineIntrinsicMacro(
        Symbol.create("defproject"),
        HaraMacro.nativeMacro(
            Symbol.create("defproject"),
            invocation -> {
              if (invocation.count() != 3
                  || !(invocation.nth(1) instanceof Symbol name)
                  || name.getNamespace() != null
                  || !(invocation.nth(2) instanceof IMapType<?, ?> options)) {
                throw new HaraException(
                    "defproject expects an unqualified project name and options map");
              }
              IMapType descriptor =
                  (IMapType)
                      ((IMapType) options)
                          .assoc(
                              Keyword.create("name"),
                              List.Standard.from(null, Symbol.create("quote"), name));
              return List.Standard.from(
                  null, Symbol.create("def"), Symbol.create("project"), descriptor);
            }));
  }

  private void installNumericBuiltins(HaraNamespace target) {
    target.define("+", new VariadicBuiltin("+", values -> arithmetic("+", values)));
    target.define("-", new VariadicBuiltin("-", values -> arithmetic("-", values)));
    target.define("*", new VariadicBuiltin("*", values -> arithmetic("*", values)));
    target.define("/", new VariadicBuiltin("/", values -> arithmetic("/", values)));
    target.define("mod", new VariadicBuiltin("mod", values -> arithmetic("mod", values)));
    target.define("compare", new VariadicBuiltin("compare", this::compareValues));
    target.define("=", new VariadicBuiltin("=", values -> compare("=", values)));
    target.define("not=", new VariadicBuiltin("not=", values -> compare("not=", values)));
    target.define("<", new VariadicBuiltin("<", values -> compare("<", values)));
    target.define("<=", new VariadicBuiltin("<=", values -> compare("<=", values)));
    target.define(">", new VariadicBuiltin(">", values -> compare(">", values)));
    target.define(">=", new VariadicBuiltin(">=", values -> compare(">=", values)));
    target.define("bigint", new UnaryBuiltin("bigint", HaraNumericConversions::toBigInteger));
    target.define("bigdec", new UnaryBuiltin("bigdec", HaraNumericConversions::toBigDecimal));
    target.define("double", new UnaryBuiltin("double", HaraNumericConversions::toDouble));
    target.define(
        "not", new UnaryBuiltin("not", value -> value == null || Boolean.FALSE.equals(value)));
    target.define("boolean?", new UnaryBuiltin("boolean?", value ->
        HaraBox.unwrap(value) instanceof Boolean));
    target.define(
        "nil?",
        new UnaryBuiltin(
            "nil?",
            value -> {
              Object unwrapped = HaraBox.unwrap(value);
              return unwrapped == null || unwrapped == HaraNull.SINGLETON;
            }));
    target.define(
        "empty?",
        new UnaryBuiltin(
            "empty?", value -> !Boolean.TRUE.equals(iterHasNext(iterValue(value)))));
    target.define("vec", new UnaryBuiltin("vec", this::toVector));
    target.define(
        "array?",
        new UnaryBuiltin("array?", value -> HaraBox.unwrap(value) instanceof HaraArray));
    target.define("reverse", new UnaryBuiltin("reverse", this::reverseValue));
    target.define(
        "map?",
        new UnaryBuiltin("map?", value -> HaraBox.unwrap(value) instanceof IMapType<?, ?>));
    target.define(
        "set?",
        new UnaryBuiltin(
            "set?",
            value -> HaraBox.unwrap(value) instanceof hara.lang.data.types.ISetType<?>));
    target.define(
        "vector?",
        new UnaryBuiltin(
            "vector?",
            value ->
                HaraBox.unwrap(value) instanceof hara.lang.data.Vector<?>
                    || HaraBox.unwrap(value) instanceof hara.lang.data.Tuple.Tup0
                    || HaraBox.unwrap(value) instanceof hara.lang.data.Tuple.Tup1<?>));
    target.define(
        "symbol?",
        new UnaryBuiltin("symbol?", value -> HaraBox.unwrap(value) instanceof Symbol));
    target.define(
        "keyword?",
        new UnaryBuiltin("keyword?", value -> HaraBox.unwrap(value) instanceof Keyword));
    target.define(
        "string?",
        new UnaryBuiltin("string?", value -> HaraBox.unwrap(value) instanceof String));
    target.define(
        "number?",
        new UnaryBuiltin("number?", value -> HaraBox.unwrap(value) instanceof Number));
    target.define("integer?", new UnaryBuiltin("integer?", value -> {
      Object raw = HaraBox.unwrap(value);
      return raw instanceof Byte || raw instanceof Short || raw instanceof Integer
          || raw instanceof Long || raw instanceof java.math.BigInteger;
    }));
    target.define("decimal?", new UnaryBuiltin("decimal?", value -> {
      Object raw = HaraBox.unwrap(value);
      return raw instanceof Float || raw instanceof Double || raw instanceof java.math.BigDecimal;
    }));
    target.define("sequential?", new UnaryBuiltin("sequential?", value ->
        HaraBox.unwrap(value) instanceof hara.lang.data.types.ISequentialType<?>));
    target.define("collection?", new UnaryBuiltin("collection?", value -> {
      Object raw = HaraBox.unwrap(value);
      return raw instanceof hara.lang.protocol.IColl<?> || raw instanceof HaraArray;
    }));
    target.define("tuple?", new UnaryBuiltin("tuple?", value ->
        HaraBox.unwrap(value) instanceof hara.lang.data.Tuple.Tup0));
    target.define("queue?", new UnaryBuiltin("queue?", value ->
        HaraBox.unwrap(value) instanceof hara.lang.data.Queue<?>));
    target.define("object?", new UnaryBuiltin("object?", value ->
        HaraBox.unwrap(value) instanceof HaraObject));
    target.define("bytes?", new UnaryBuiltin("bytes?", value ->
        HaraBox.unwrap(value) instanceof byte[]));
    target.define("atom?", new UnaryBuiltin("atom?", value ->
        HaraBox.unwrap(value) instanceof hara.lang.data.Atom<?>));
    target.define("promise?", new UnaryBuiltin("promise?", value ->
        HaraBox.unwrap(value) instanceof HaraPromise));
    target.define("coroutine?", new UnaryBuiltin("coroutine?", value ->
        HaraBox.unwrap(value) instanceof StdFoundationCoroutine.HaraCoroutine));
    target.define("callable?", new UnaryBuiltin("callable?", this::isFunctionValue));
    target.define("iterator?", new UnaryBuiltin("iterator?", value ->
        HaraBox.unwrap(value) instanceof Iterator<?>));
    target.define("iterable?", new UnaryBuiltin("iterable?", value -> {
      Object raw = HaraBox.unwrap(value);
      return raw instanceof Iterable<?> || raw instanceof Iterator<?> || raw instanceof byte[];
    }));
    target.define("counted?", new UnaryBuiltin("counted?", value ->
        HaraBox.unwrap(value) instanceof ICount));
    target.define("indexed?", new UnaryBuiltin("indexed?", value ->
        HaraBox.unwrap(value) instanceof hara.lang.protocol.INth<?>));
    target.define("associative?", new UnaryBuiltin("associative?", value ->
        HaraBox.unwrap(value) instanceof hara.lang.protocol.IAssoc<?, ?>));
    target.define("derefable?", new UnaryBuiltin("derefable?", value ->
        HaraBox.unwrap(value) instanceof IDeref<?>));
    target.define("watchable?", new UnaryBuiltin("watchable?", value ->
        HaraBox.unwrap(value) instanceof hara.lang.protocol.IWatch<?, ?>));
    target.define(
        "instance?",
        new VariadicBuiltin(
            "instance?",
            values -> {
              if (values.length != 2) {
                throw new HaraException("instance? expects a Hara type and value");
              }
              Object type = HaraBox.unwrap(values[0]);
              Object value = HaraBox.unwrap(values[1]);
              if (!(type instanceof HaraType)) {
                throw new HaraException("instance? expects a Hara type");
              }
              return value instanceof HaraStruct struct && struct.type() == type;
            }));
    target.define(
        "symbol",
        new VariadicBuiltin(
            "symbol",
            values -> {
              if (values.length == 1) {
                Object value = HaraBox.unwrap(values[0]);
                if (value instanceof Symbol) return value;
                if (value instanceof Keyword keyword) {
                  return Symbol.create(keyword.display().substring(1));
                }
                if (value instanceof String text) return Symbol.create(text);
              } else if (values.length == 2) {
                return Symbol.create(
                    String.valueOf(HaraBox.unwrap(values[0])),
                    String.valueOf(HaraBox.unwrap(values[1])));
              }
              throw new HaraException("symbol expects a name or namespace and name");
            }));
    target.define(
        "keyword",
        new UnaryBuiltin(
            "keyword",
            value -> {
              Object unwrapped = HaraBox.unwrap(value);
              if (unwrapped instanceof Keyword) return unwrapped;
              if (unwrapped instanceof Symbol symbol) return Keyword.create(symbol.display());
              if (unwrapped instanceof String text) return Keyword.create(text);
              throw new HaraException("keyword expects a name");
            }));
    target.define(
        "ex-info",
        new VariadicBuiltin(
            "ex-info",
            values -> {
              if (values.length < 2
                  || values.length > 3
                  || !(HaraBox.unwrap(values[0]) instanceof String)
                  || !(HaraBox.unwrap(values[1]) instanceof IMetadata)) {
                throw new HaraException("ex-info expects a message, metadata map, and optional cause");
              }
              Throwable cause =
                  values.length == 3 && HaraBox.unwrap(values[2]) instanceof Throwable
                      ? (Throwable) HaraBox.unwrap(values[2])
                      : null;
              return new hara.lang.base.Ex.Info(
                  (String) HaraBox.unwrap(values[0]),
                  (IMetadata) HaraBox.unwrap(values[1]),
                  cause);
            }));
    target.define(
        "ex-data",
        new UnaryBuiltin(
            "ex-data",
            value -> {
              Object unwrapped = HaraBox.unwrap(value);
              return unwrapped instanceof hara.lang.protocol.IExInfo
                  ? ((hara.lang.protocol.IExInfo) unwrapped).getData()
                  : null;
            }));
    target.define(
        "ex-message",
        new UnaryBuiltin(
            "ex-message",
            value -> {
              Object unwrapped = HaraBox.unwrap(value);
              return unwrapped instanceof Throwable
                  ? ((Throwable) unwrapped).getMessage()
                  : String.valueOf(unwrapped);
            }));
    target.define(
        "ex-class",
        new UnaryBuiltin(
            "ex-class",
            value -> {
              Object unwrapped = HaraBox.unwrap(value);
              return unwrapped == null ? null : unwrapped.getClass().getName();
            }));
    target.define("load-string", new UnaryBuiltin("load-string", this::loadString));
    target.define("eval", new UnaryBuiltin("eval", this::evalForm));
    target.define("load-file", new UnaryBuiltin("load-file", this::loadFile));
    target.define("load-resource", new UnaryBuiltin("load-resource", this::loadResource));
    target.define("read-forms", new VariadicBuiltin("read-forms", this::readForms));
    target.define("require", new VariadicBuiltin("require", this::requireModule));
    target.define("refer", new UnaryBuiltin("refer", this::referNamespace));
    target.define(
        "meta",
        new UnaryBuiltin(
            "meta",
            value -> {
              Object unwrapped = HaraBox.unwrap(value);
              if (unwrapped == null || unwrapped == HaraNull.SINGLETON) return null;
              if (unwrapped instanceof HaraVar) return ((HaraVar) unwrapped).meta();
              if (unwrapped instanceof hara.lang.protocol.IObjType) {
                return ((hara.lang.protocol.IObjType) unwrapped).meta();
              }
              return null;
            }));
    target.define(
        "with-meta",
        new VariadicBuiltin(
            "with-meta",
            values -> {
              if (values.length != 2) {
                throw new HaraException("with-meta expects a value and metadata");
              }
              return protocolCall("IObjType", "with-meta", values);
            }));
    target.define(
        "name",
        new UnaryBuiltin(
            "name", value -> protocolCall("INamespaced", "name", new Object[] {value})));
    target.define(
        "namespace",
        new UnaryBuiltin(
            "namespace",
            value -> protocolCall("INamespaced", "namespace", new Object[] {value})));
    target.define("in-ns", new UnaryBuiltin("in-ns", this::inNamespace));
    target.define("the-ns", new UnaryBuiltin("the-ns", this::theNamespace));
    target.define("ns-name", new UnaryBuiltin("ns-name", this::namespaceName));
    target.define("ns-publics", new UnaryBuiltin("ns-publics", this::namespacePublics));
    target.define("ns-aliases", new UnaryBuiltin("ns-aliases", this::namespaceAliases));
    target.define(
        "resolve",
        new UnaryBuiltin(
            "resolve",
            value -> {
              Object raw = HaraBox.unwrap(value);
              if (!(raw instanceof Symbol symbol)) {
                throw new HaraException("resolve expects a symbol");
              }
              return resolve(symbol);
            }));
    target.define(
        "special-symbol?",
        new UnaryBuiltin(
            "special-symbol?",
            value -> {
              Object raw = HaraBox.unwrap(value);
              return raw instanceof Symbol symbol
                  && symbol.getNamespace() == null
                  && SPECIAL_SYMBOLS.contains(symbol.getName());
            }));
    target.define(
        "requiring-resolve", new UnaryBuiltin("requiring-resolve", this::requiringResolve));
    target.define(
        "current-namespace",
        new VariadicBuiltin(
            "current-namespace",
            values -> {
              requireMethodArity("current-namespace", values, 0);
              return currentNamespace.name();
            }));
    target.define(
        "current-symbols",
        new VariadicBuiltin(
            "current-symbols",
            values -> {
              requireMethodArity("current-symbols", values, 0);
              return currentSymbolNames().toArray();
            }));
    target.define("use", new UnaryBuiltin("use", this::useNamespace));
    target.define("iter", new UnaryBuiltin("iter", this::iterValue));
    target.define("seq", new VariadicBuiltin("seq", this::seqValue));
    target.define("seq?", new UnaryBuiltin("seq?", this::isSeq));
    target.define("iter?", new UnaryBuiltin("iter?", this::isIterator));
    target.define("iter-has?", new UnaryBuiltin("iter-has?", this::iterHasNext));
    target.define("iter-next", new UnaryBuiltin("iter-next", this::iterNext));
    target.define("iter-close", new UnaryBuiltin("iter-close", this::iterClose));
    target.define("concat", new VariadicBuiltin("concat", this::concatIterators));
    target.define("iter-map", new VariadicBuiltin("iter-map", this::iterMap));
    target.define("iter-filter", new VariadicBuiltin("iter-filter", this::iterFilter));
    target.define("iter-take-while", new VariadicBuiltin("iter-take-while", this::iterTakeWhile));
    target.define("iter-drop-while", new VariadicBuiltin("iter-drop-while", this::iterDropWhile));
    target.define("iter-mapcat", new VariadicBuiltin("iter-mapcat", this::iterMapcat));
    target.define("iter-keep", new VariadicBuiltin("iter-keep", this::iterKeep));
    target.define("iter-interpose", new VariadicBuiltin("iter-interpose", this::iterInterpose));
    target.define("iter-interleave", new VariadicBuiltin("iter-interleave", this::iterInterleave));
    target.define("iter-every?", new VariadicBuiltin("iter-every?", this::iterEvery));
    target.define("iter-any?", new VariadicBuiltin("iter-any?", this::iterAny));
    target.define("iter-take", new VariadicBuiltin("iter-take", this::iterTake));
    target.define("iter-drop", new VariadicBuiltin("iter-drop", this::iterDrop));
    target.define("iter-zip", new VariadicBuiltin("iter-zip", this::iterZip));
    target.define("iter-cycle", new UnaryBuiltin("iter-cycle", this::iterCycle));
    target.define(
        "iter-partition-pair", new UnaryBuiltin("iter-partition-pair", this::iterPartitionPair));
    target.define(
        "iter-partition-all",
        new VariadicBuiltin("iter-partition-all", values -> iterPartition(values, true)));
    target.define(
        "iter-partition",
        new VariadicBuiltin("iter-partition", values -> iterPartition(values, false)));
    target.define("iter-range", new VariadicBuiltin("iter-range", this::iterRange));
    target.define("iter-constantly", new UnaryBuiltin("iter-constantly", Iter::constantly));
    target.define("iter-repeatedly", new UnaryBuiltin("iter-repeatedly", this::iterRepeatedly));
    target.define("iter-iterate", new VariadicBuiltin("iter-iterate", this::iterIterate));
    target.define("alter-var-root", new VariadicBuiltin("alter-var-root", this::alterVarRoot));
    target.define("apply", new VariadicBuiltin("apply", this::applyFunction));
    target.define("module-revision", new UnaryBuiltin("module-revision", this::moduleRevision));
    target.define(
        "module-dependencies", new UnaryBuiltin("module-dependencies", this::moduleDependencies));
    target.define(
        "count",
        new UnaryBuiltin("count", value -> protocolCall("ICount", "count", new Object[] {value})));
    target.define(
        "get", new VariadicBuiltin("get", values -> protocolCall("ILookup", "lookup", values)));
    target.define(
        "find", new VariadicBuiltin("find", values -> protocolCall("IFind", "find", values)));
    target.define(
        "has?", new VariadicBuiltin("has?", values -> protocolCall("IFind", "has?", values)));
    target.define(
        "assoc", new VariadicBuiltin("assoc", this::associateValues));
    target.define("conj", new VariadicBuiltin("conj", this::conjoin));
    target.define(
        "cons",
        new VariadicBuiltin(
            "cons",
            values -> {
              if (values.length != 2) {
                throw new HaraException("cons expects an item and a collection");
              }
              return protocolCall("ICons", "cons", new Object[] {values[1], values[0]});
            }));
    target.define("nth", new VariadicBuiltin("nth", values -> protocolCall("INth", "nth", values)));
    target.define(
        "empty",
        new UnaryBuiltin("empty", value -> protocolCall("IEmpty", "empty", new Object[] {value})));
    target.define(
        "dissoc",
        new VariadicBuiltin(
            "dissoc",
            values -> {
              if (values.length < 1) {
                throw new HaraException("dissoc expects a collection and at least one key");
              }
              Object result = values[0];
              for (int i = 1; i < values.length; i++) {
                if (HaraBox.unwrap(result) == null
                    || HaraBox.unwrap(result) == HaraNull.SINGLETON) {
                  return result;
                }
                result = protocolCall("IDissoc", "dissoc", new Object[] {result, values[i]});
              }
              return result;
            }));
    target.define(
        "peek",
        new UnaryBuiltin(
            "peek", value -> protocolCall("INavigation", "peek-first", new Object[] {value})));
    target.define(
        "pop",
        new UnaryBuiltin(
            "pop", value -> protocolCall("INavigation", "pop-first", new Object[] {value})));
  }


  @SuppressWarnings({"rawtypes", "unchecked"})
  static Object lookupValue(IMapType<?, ?> map, Object key) {
    return ((IMapType) map).lookup(key);
  }

  private Object conjoin(Object[] values) {
    Object result;
    int firstValue;
    if (values.length < 2) {
      result = BuiltinStruct.vector(new Object[0]);
      firstValue = 0;
    } else {
      result = values[0];
      firstValue = 1;
    }
    for (int i = firstValue; i < values.length; i++) {
      result = protocolCall("IConj", "conj", new Object[] {result, values[i]});
    }
    return result;
  }

  private Object associateValues(Object[] values) {
    if (values.length < 3 || values.length % 2 == 0) {
      throw new HaraException("assoc expects a collection and key/value pairs");
    }
    Object result = values[0];
    for (int i = 1; i < values.length; i += 2) {
      result = protocolCall("IAssoc", "assoc", new Object[] {result, values[i], values[i + 1]});
    }
    return result;
  }

  private Object toVector(Object value) {
    ArrayList<Object> elements = new ArrayList<>();
    Iterator<?> iterator = (Iterator<?>) iterValue(value);
    while (iterator.hasNext()) elements.add(iterator.next());
    return hara.lang.data.Vector.Standard.from(null, elements.toArray());
  }

  private Object reverseValue(Object value) {
    ArrayList<Object> elements = new ArrayList<>();
    Iterator<?> iterator = (Iterator<?>) iterValue(value);
    while (iterator.hasNext()) elements.add(iterator.next());
    java.util.Collections.reverse(elements);
    return hara.lang.data.Vector.Standard.from(null, elements.toArray());
  }

  Object mapValues(Object[] values) {
    if (values.length == 1) {
      Object function = values[0];
      return new VariadicBuiltin(
          "map-transform",
          inputs -> {
            if (inputs.length == 0) {
              throw new HaraException("map transform expects at least one collection");
            }
            Object[] arguments = new Object[inputs.length + 1];
            arguments[0] = function;
            System.arraycopy(inputs, 0, arguments, 1, inputs.length);
            Object mapped = iterMap(arguments);
            ArrayList<Object> output = new ArrayList<>();
            Iterator<?> iterator = (Iterator<?>) mapped;
            while (iterator.hasNext()) output.add(iterator.next());
            return inputs[0] instanceof HaraArray
                ? new HaraArray(output.toArray())
                : hara.lang.data.Vector.Standard.from(null, output.toArray());
          });
    }
    if (values.length < 2) throw new HaraException("map expects a function and collections");
    return seqValue(
        new Object[] {
          iterMap(values)
        });
  }

  Object partitionValues(Object[] values, boolean includePartial) {
    if (values.length == 1) {
      Object amount = values[0];
      return new UnaryBuiltin(
          includePartial ? "partition-all-transform" : "partition-transform",
          input -> {
            Object partitioned = iterPartition(new Object[] {amount, input}, includePartial);
            ArrayList<Object> output = new ArrayList<>();
            Iterator<?> iterator = (Iterator<?>) partitioned;
            while (iterator.hasNext()) output.add(iterator.next());
            return input instanceof HaraArray
                ? new HaraArray(output.toArray())
                : hara.lang.data.Vector.Standard.from(null, output.toArray());
          });
    }
    requireMethodArity(includePartial ? "partition-all" : "partition", values, 2);
    return seqValue(new Object[] {iterPartition(values, includePartial)});
  }

  Object removeValues(Object[] values) {
    requireMethodArity("remove", values, 2);
    Object predicate = values[0];
    Iterator<?> iterator = (Iterator<?>) iterValue(values[1]);
    ArrayList<Object> kept = new ArrayList<>();
    while (iterator.hasNext()) {
      Object item = iterator.next();
      Object result = invokeCallable(predicate, new Object[] {item});
      if (result == null || Boolean.FALSE.equals(result)) kept.add(item);
    }
    return hara.lang.data.Vector.Standard.from(null, kept.toArray());
  }

  Object reduceKeyValues(Object[] values) {
    requireMethodArity("reduce-kv", values, 3);
    Object result = values[1];
    Iterator<?> iterator = (Iterator<?>) iterValue(values[2]);
    while (iterator.hasNext()) {
      Object entry = iterator.next();
      Object key;
      Object value;
      if (entry instanceof java.util.Map.Entry<?, ?> mapEntry) {
        key = mapEntry.getKey();
        value = mapEntry.getValue();
      } else if (entry instanceof hara.lang.protocol.IPair<?, ?> pair) {
        key = pair.getKey();
        value = pair.getValue();
      } else {
        key = protocolCall("INth", "nth", new Object[] {entry, 0L});
        value = protocolCall("INth", "nth", new Object[] {entry, 1L});
      }
      result = HaraBox.unwrap(invokeCallable(values[0], new Object[] {result, key, value}));
    }
    return result;
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  Object mergeMaps(Object[] values) {
    IMapType result = hara.lang.data.Map.Standard.EMPTY;
    for (Object raw : values) {
      Object value = HaraBox.unwrap(raw);
      if (value == null || value == HaraNull.SINGLETON) continue;
      if (!(value instanceof IMapType<?, ?> map)) {
        throw new HaraException("merge expects maps");
      }
      for (Object entryObject : map) {
        java.util.Map.Entry<?, ?> entry = (java.util.Map.Entry<?, ?>) entryObject;
        result = (IMapType) result.assoc(entry.getKey(), entry.getValue());
      }
    }
    return result;
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  Object selectKeys(Object[] values) {
    requireMethodArity("select-keys", values, 2);
    Object raw = HaraBox.unwrap(values[0]);
    if (!(raw instanceof IMapType<?, ?> map)) {
      throw new HaraException("select-keys expects a map and keys");
    }
    IMapType result = hara.lang.data.Map.Standard.EMPTY;
    Iterator<?> keys = (Iterator<?>) iterValue(values[1]);
    while (keys.hasNext()) {
      Object key = keys.next();
      if (((IMapType) map).has(key)) {
        result = (IMapType) result.assoc(key, ((IMapType) map).lookup(key));
      }
    }
    return result;
  }

  private static String concatenateStrings(Object[] values) {
    StringBuilder result = new StringBuilder();
    for (Object value : values) {
      Object unwrapped = HaraBox.unwrap(value);
      if (unwrapped == null || unwrapped == HaraNull.SINGLETON) {
        continue;
      }
      if (unwrapped instanceof IDisplay) {
        result.append(((IDisplay) unwrapped).display());
      } else {
        result.append(unwrapped);
      }
    }
    return result.toString();
  }

  private void installCoreBuiltins(HaraNamespace target) {
    target.define("str", new VariadicBuiltin("str", HaraContext::concatenateStrings));
    target.define(
        "list",
        new VariadicBuiltin(
            "list", values -> hara.lang.data.List.Standard.from(null, values)));
    target.define(
        "atom",
        new UnaryBuiltin(
            "atom", value -> new hara.lang.data.Atom.Standard<>(HaraBox.unwrap(value))));
    target.define(
        "reset!",
        new VariadicBuiltin(
            "reset!",
            values -> {
              if (values.length != 2) {
                throw new HaraException("reset! expects a reference and value");
              }
              return protocolCall("IReset", "reset", values);
            }));
    target.define("swap!", new VariadicBuiltin("swap!", this::swapAtom));
    target.define(
        "compare-and-set!", new VariadicBuiltin("compare-and-set!", this::compareAndSetAtom));
    target.define("add-watch", new VariadicBuiltin("add-watch", this::addAtomWatch));
    target.define("remove-watch", new VariadicBuiltin("remove-watch", this::removeAtomWatch));
    target.define("get-watches", new UnaryBuiltin("get-watches", this::getAtomWatches));
    target.define("gensym", new VariadicBuiltin("gensym", values -> {
      if (values.length > 1) throw new HaraException("gensym expects zero or one prefix");
      return gensym(values.length == 0 ? null : String.valueOf(HaraBox.unwrap(values[0])));
    }));
    target.define(
        "macroexpand-1", new UnaryBuiltin("macroexpand-1", value -> macroExpand(value, false)));
    target.define(
        "pr-str",
        new UnaryBuiltin(
            "pr-str",
            value -> hara.kernel.builtin.BuiltinUtil.prStr(HaraBox.unwrap(value))));
    target.define(
        "char?",
        new UnaryBuiltin("char?", value -> HaraBox.unwrap(value) instanceof Character));
    target.define(
        "uuid?",
        new UnaryBuiltin("uuid?", value -> HaraBox.unwrap(value) instanceof java.util.UUID));
    target.define(
        "regexp?",
        new UnaryBuiltin(
            "regexp?", value -> HaraBox.unwrap(value) instanceof java.util.regex.Pattern));
    target.define(
        "map-entry?",
        new UnaryBuiltin(
            "map-entry?",
            value -> {
              Object raw = HaraBox.unwrap(value);
              return raw instanceof hara.lang.protocol.IPair<?, ?>
                  || raw instanceof java.util.Map.Entry<?, ?>;
            }));
    target.define(
        "list?",
        new UnaryBuiltin("list?", value -> HaraBox.unwrap(value) instanceof hara.lang.data.List<?>));
    target.define("promise", new UnaryBuiltin("promise", this::promiseRun));
    target.define("bytes", new VariadicBuiltin("bytes", this::createBytes));
    target.define("array", new VariadicBuiltin("array", HaraArray::new));
    target.define("object", new VariadicBuiltin("object", HaraObject::new));
    target.define("bit-and", new VariadicBuiltin("bit-and", values -> bitOperation("and", values)));
    target.define("bit-or", new VariadicBuiltin("bit-or", values -> bitOperation("or", values)));
    target.define("bit-xor", new VariadicBuiltin("bit-xor", values -> bitOperation("xor", values)));
    target.define("bit-not", new UnaryBuiltin("bit-not", value -> (long) ~int32(value, "bit-not")));
    target.define(
        "bit-shift-left", new VariadicBuiltin("bit-shift-left", values -> bitShift(values, true)));
    target.define(
        "bit-shift-right",
        new VariadicBuiltin("bit-shift-right", values -> bitShift(values, false)));
  }

  private Object createBytes(Object[] values) {
    byte[] result = new byte[values.length];
    for (int i = 0; i < values.length; i++) {
      result[i] = (byte) byteNumber(values[i], "bytes");
    }
    return result;
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private Object swapAtom(Object[] values) {
    if (values.length < 2) throw new HaraException("swap! expects an atom, function, and arguments");
    Object raw = HaraBox.unwrap(values[0]);
    if (!(raw instanceof hara.lang.data.Atom.Swap swap)) {
      throw new HaraException("swap! expects an atom");
    }
    for (;;) {
      Object oldValue = swap.deref();
      Object[] arguments = new Object[values.length - 1];
      arguments[0] = oldValue;
      if (values.length > 2) System.arraycopy(values, 2, arguments, 1, values.length - 2);
      Object newValue = HaraBox.unwrap(invokeCallable(values[1], arguments));
      swap.validate(newValue);
      if (swap.cas(oldValue, newValue)) {
        swap.notifyWatches(oldValue, newValue);
        return newValue;
      }
    }
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private Object compareAndSetAtom(Object[] values) {
    requireMethodArity("compare-and-set!", values, 3);
    Object raw = HaraBox.unwrap(values[0]);
    if (!(raw instanceof hara.lang.data.Atom.Swap swap)) {
      throw new HaraException("compare-and-set! expects an atom");
    }
    Object oldValue = HaraBox.unwrap(values[1]);
    Object newValue = HaraBox.unwrap(values[2]);
    swap.validate(newValue);
    boolean changed = swap.cas(oldValue, newValue);
    if (changed) swap.notifyWatches(oldValue, newValue);
    return changed;
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private Object addAtomWatch(Object[] values) {
    requireMethodArity("add-watch", values, 3);
    Object reference = HaraBox.unwrap(values[0]);
    if (!(reference instanceof hara.lang.protocol.IWatch watch)) {
      throw new HaraException("add-watch expects a watchable reference");
    }
    Object key = HaraBox.unwrap(values[1]);
    Object callback = values[2];
    watch.addWatch(key, entry -> invokeCallable(callback, new Object[] {
        key, reference, ((hara.lang.protocol.IWatch.WatchEntry) entry).oldVal(),
        ((hara.lang.protocol.IWatch.WatchEntry) entry).newVal()}));
    synchronized (guestWatches) {
      guestWatches.computeIfAbsent(reference, ignored -> new LinkedHashMap<>()).put(key, callback);
    }
    return reference;
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private Object removeAtomWatch(Object[] values) {
    requireMethodArity("remove-watch", values, 2);
    Object reference = HaraBox.unwrap(values[0]);
    if (!(reference instanceof hara.lang.protocol.IWatch watch)) {
      throw new HaraException("remove-watch expects a watchable reference");
    }
    Object key = HaraBox.unwrap(values[1]);
    watch.removeWatch(key);
    synchronized (guestWatches) {
      LinkedHashMap<Object, Object> watches = guestWatches.get(reference);
      if (watches != null) {
        watches.remove(key);
        if (watches.isEmpty()) guestWatches.remove(reference);
      }
    }
    return reference;
  }

  private Object getAtomWatches(Object value) {
    Object reference = HaraBox.unwrap(value);
    if (!(reference instanceof hara.lang.protocol.IWatch)) {
      throw new HaraException("get-watches expects a watchable reference");
    }
    ArrayList<Object> entries = new ArrayList<>();
    synchronized (guestWatches) {
      for (Map.Entry<Object, Object> entry :
          guestWatches.getOrDefault(reference, new LinkedHashMap<>()).entrySet()) {
        entries.add(entry.getKey());
        entries.add(entry.getValue());
      }
    }
    return hara.lang.data.Map.Standard.from(null, entries.toArray());
  }

  private void installNativeLibraries() {
    HaraNamespace jvm = namespace("hara.native.jvm");
    jvm.define(
        "set!",
        new VariadicBuiltin(
            "hara.native.jvm/set!",
            values -> {
              if (values.length != 3)
                throw new HaraException("hara.native.jvm/set! expects 3 arguments");
              String member;
              if (values[1] instanceof Symbol) member = ((Symbol) values[1]).getName();
              else if (values[1] instanceof Keyword) member = ((Keyword) values[1]).getName();
              else if (values[1] instanceof String) member = (String) values[1];
              else throw new HaraException("JVM member must be a symbol, keyword, or string");
              NativeFlavorProvider provider = nativeProvider();
              if (provider == null) {
                throw new HaraException("hara.native.jvm/set! requires an ns :flavor declaration");
              }
              return provider.writeMember(
                  HaraBox.unwrap(values[0]), member, HaraBox.unwrap(values[2]), nativeAccess());
            }));
    namespace("hara.native.jvm.reflect");
    namespace("hara.native.jvm.classpath");
    namespace("hara.native.jvm.compiler");
    installJvmNativeLibraries();
  }

  private void installJvmNativeLibraries() {
    HaraNamespace reflect = namespace("hara.native.jvm.reflect");
    reflect.define(
        "type",
        new UnaryBuiltin(
            "reflect/type", value -> jvmProvider().type(HaraBox.unwrap(value), nativeAccess())));
    reflect.define(
        "name",
        new UnaryBuiltin(
            "reflect/name",
            value -> jvmProvider().typeName(HaraBox.unwrap(value), nativeAccess())));
    reflect.define(
        "instance?",
        new VariadicBuiltin(
            "reflect/instance?",
            values -> {
              requireMethodArity("reflect/instance?", values, 2);
              return jvmProvider()
                  .isInstance(HaraBox.unwrap(values[0]), HaraBox.unwrap(values[1]), nativeAccess());
            }));
    reflect.define(
        "fields",
        new UnaryBuiltin(
            "reflect/fields",
            value -> jvmProvider().fields(HaraBox.unwrap(value), nativeAccess())));
    reflect.define(
        "methods",
        new UnaryBuiltin(
            "reflect/methods",
            value -> jvmProvider().methods(HaraBox.unwrap(value), nativeAccess())));

    HaraNamespace classpath = namespace("hara.native.jvm.classpath");
    classpath.define(
        "paths",
        new VariadicBuiltin(
            "classpath/paths",
            values -> {
              requireMethodArity("classpath/paths", values, 0);
              return jvmProvider().classPath(nativeAccess());
            }));
    classpath.define(
        "add!",
        new UnaryBuiltin(
            "classpath/add!",
            value ->
                jvmProvider().addClassPath(String.valueOf(HaraBox.unwrap(value)), nativeAccess())));

    HaraNamespace compiler = namespace("hara.native.jvm.compiler");
    compiler.define(
        "compile",
        new UnaryBuiltin(
            "compiler/compile",
            value -> jvmProvider().compile(HaraBox.unwrap(value), nativeAccess())));
    compiler.define(
        "define!",
        new UnaryBuiltin(
            "compiler/define!",
            value -> {
              Object bytecode = HaraBox.unwrap(value);
              if (!(bytecode instanceof byte[])) {
                throw new HaraException("compiler/define! expects bytes");
              }
              return jvmProvider().defineClass((byte[]) bytecode, nativeAccess());
            }));
    compiler.define(
        "compile!",
        new UnaryBuiltin(
            "compiler/compile!",
            value -> {
              JvmFlavorProvider provider = jvmProvider();
              return provider.defineClass(
                  provider.compile(HaraBox.unwrap(value), nativeAccess()), nativeAccess());
            }));
  }

  void installStringLibrary() {
    withDefinitionOrigin(HaraVar.Origin.JAVA_LIBRARY, this::defineStringLibrary);
  }

  private void defineStringLibrary() {
    HaraNamespace string = namespace("std.foundation.string");

    // Spec-named symbols.
    string.define(
        "length",
        new UnaryBuiltin("str/length", value -> (long) codePointLength(stringValue(value, "str/length"))));
    string.define(
        "blank?",
        new UnaryBuiltin(
            "str/blank?", value -> stringValue(value, "str/blank?").trim().isEmpty()));
    string.define(
        "includes?",
        new VariadicBuiltin(
            "str/includes?",
            values -> {
              String[] pair = stringPair(values, "str/includes?");
              return pair[0].contains(pair[1]);
            }));
    string.define(
        "starts-with?",
        new VariadicBuiltin(
            "str/starts-with?",
            values -> {
              String[] pair = stringPair(values, "str/starts-with?");
              return pair[0].startsWith(pair[1]);
            }));
    string.define(
        "ends-with?",
        new VariadicBuiltin(
            "str/ends-with?",
            values -> {
              String[] pair = stringPair(values, "str/ends-with?");
              return pair[0].endsWith(pair[1]);
            }));
    string.define("char-at", new VariadicBuiltin("str/char-at", this::stringCharAt));
    string.define("slice", new VariadicBuiltin("str/slice", this::stringSlice));
    string.define("index-of", new VariadicBuiltin("str/index-of", this::stringIndexOf));
    string.define(
        "last-index-of", new VariadicBuiltin("str/last-index-of", this::stringLastIndexOf));
    string.define("split", new VariadicBuiltin("str/split", this::stringSplit));
    string.define("split-lines", new VariadicBuiltin("str/split-lines", this::stringSplitLines));
    string.define("join", new VariadicBuiltin("str/join", this::stringJoin));
    string.define("repeat", new VariadicBuiltin("str/repeat", this::stringRepeat));
    string.define("replace", new VariadicBuiltin("str/replace", this::stringReplace));
    string.define(
        "replace-first", new VariadicBuiltin("str/replace-first", this::stringReplaceFirst));
    string.define(
        "trim", new UnaryBuiltin("str/trim", value -> stringValue(value, "str/trim").trim()));
    string.define(
        "trim-left",
        new UnaryBuiltin(
            "str/trim-left", value -> stringValue(value, "str/trim-left").stripLeading()));
    string.define(
        "trim-right",
        new UnaryBuiltin(
            "str/trim-right", value -> stringValue(value, "str/trim-right").stripTrailing()));
    string.define(
        "upper",
        new UnaryBuiltin(
            "str/upper", value -> stringValue(value, "str/upper").toUpperCase(java.util.Locale.ROOT)));
    string.define(
        "lower",
        new UnaryBuiltin(
            "str/lower", value -> stringValue(value, "str/lower").toLowerCase(java.util.Locale.ROOT)));
    string.define(
        "capitalize",
        new UnaryBuiltin("str/capitalize", value -> stringCapitalize(stringValue(value, "str/capitalize"))));
    string.define(
        "decapitalize",
        new UnaryBuiltin(
            "str/decapitalize", value -> stringDecapitalize(stringValue(value, "str/decapitalize"))));
    string.define(
        "pad-left", new VariadicBuiltin("str/pad-left", values -> padString(values, true)));
    string.define(
        "pad-right", new VariadicBuiltin("str/pad-right", values -> padString(values, false)));
    string.define(
        "reverse",
        new UnaryBuiltin(
            "str/reverse", value -> new StringBuilder(stringValue(value, "str/reverse")).reverse().toString()));
    string.define(
        "encode-utf8",
        new UnaryBuiltin(
            "str/encode-utf8",
            value -> stringValue(value, "str/encode-utf8").getBytes(StandardCharsets.UTF_8)));
    string.define(
        "decode-utf8",
        new UnaryBuiltin(
            "str/decode-utf8",
            value -> new String(bytesValue(value, "str/decode-utf8"), StandardCharsets.UTF_8)));

    // Legacy aliases (not part of the spec surface).
    string.define(
        "len", new UnaryBuiltin("str/len", value -> (long) codePointLength(stringValue(value, "str/len"))));
    string.define("char", new VariadicBuiltin("str/char", this::stringCharAt));
    string.define("substring", new VariadicBuiltin("str/substring", this::stringSlice));
    string.define(
        "to-upper",
        new UnaryBuiltin(
            "str/to-upper",
            value -> stringValue(value, "str/to-upper").toUpperCase(java.util.Locale.ROOT)));
    string.define(
        "to-lower",
        new UnaryBuiltin(
            "str/to-lower",
            value -> stringValue(value, "str/to-lower").toLowerCase(java.util.Locale.ROOT)));
    string.define(
        "encode",
        new UnaryBuiltin(
            "str/encode",
            value -> stringValue(value, "str/encode").getBytes(StandardCharsets.UTF_8)));
    string.define(
        "decode",
        new UnaryBuiltin(
            "str/decode",
            value -> new String(bytesValue(value, "str/decode"), StandardCharsets.UTF_8)));

    // Comparison helpers (not in the spec symbol list).
    string.define("comp", new VariadicBuiltin("str/comp", this::stringCompare));
    string.define(
        "lt?", new VariadicBuiltin("str/lt?", values -> ((Long) stringCompare(values)) < 0));
    string.define(
        "gt?", new VariadicBuiltin("str/gt?", values -> ((Long) stringCompare(values)) > 0));
    string.define("to-fixed", new VariadicBuiltin("str/to-fixed", this::stringToFixed));
  }

  void installBytesLibrary() {
    withDefinitionOrigin(HaraVar.Origin.JAVA_LIBRARY, this::defineBytesLibrary);
  }

  private void defineBytesLibrary() {
    HaraNamespace bytes = namespace("std.foundation.bytes");
    bytes.define(
        "count",
        new UnaryBuiltin("bytes/count", value -> (long) bytesValue(value, "bytes/count").length));
    bytes.define("get", new VariadicBuiltin("bytes/get", this::bytesGet));
    bytes.define("set", new VariadicBuiltin("bytes/set", this::bytesSet));
    bytes.define(
        "copy", new UnaryBuiltin("bytes/copy", value -> bytesValue(value, "bytes/copy").clone()));
    bytes.define("slice", new VariadicBuiltin("bytes/slice", this::bytesSlice));
    bytes.define(
        "u8", new UnaryBuiltin("bytes/u8", value -> (long) (byteNumber(value, "bytes/u8") & 0xff)));
    bytes.define(
        "s8", new UnaryBuiltin("bytes/s8", value -> (long) (byte) byteNumber(value, "bytes/s8")));
  }

  void installPromiseLibrary() {
    withDefinitionOrigin(HaraVar.Origin.JAVA_LIBRARY, this::definePromiseLibrary);
  }

  private void definePromiseLibrary() {
    HaraNamespace promise = namespace("std.foundation.promise");
    promise.define("new", new UnaryBuiltin("promise/new", this::promiseNew));
    promise.define("from", new UnaryBuiltin("promise/from", this::promiseFrom));
    promise.define("all", new UnaryBuiltin("promise/all", this::promiseAll));
    promise.define(
        "state",
        new UnaryBuiltin("promise/state", value -> requirePromise(value, "promise/state").state()));
    promise.define(
        "value",
        new UnaryBuiltin("promise/value", value -> requirePromise(value, "promise/value").value()));
    promise.define(
        "then", new VariadicBuiltin("promise/then", values -> promiseThen(values, false)));
    promise.define(
        "catch", new VariadicBuiltin("promise/catch", values -> promiseThen(values, true)));
    promise.define("finally", new VariadicBuiltin("promise/finally", this::promiseFinally));
    promise.define(
        "cancel",
        new UnaryBuiltin(
            "promise/cancel", value -> requirePromise(value, "promise/cancel").cancel()));
    promise.define("delay", new VariadicBuiltin("promise/delay", this::promiseDelay));
  }

  void installFileLibrary() {
    withDefinitionOrigin(HaraVar.Origin.JAVA_LIBRARY, this::defineFileLibrary);
  }

  void installJsonLibrary() {
    withDefinitionOrigin(HaraVar.Origin.JAVA_LIBRARY, this::defineJsonLibrary);
  }

  private void defineJsonLibrary() {
    HaraNamespace json = namespace("std.foundation.json");
    json.define(
        "read",
        new UnaryBuiltin(
            "json/read",
            value -> {
              if (!(HaraBox.unwrap(value) instanceof String source)) {
                throw new HaraException("json/read expects a string");
              }
              try {
                return hara.verify.json.StrictJson.parse(source);
              } catch (IllegalArgumentException error) {
                throw new HaraException("json/read: " + error.getMessage());
              }
            }));
    json.define(
        "write",
        new UnaryBuiltin(
            "json/write",
            value -> {
              try {
                return StdJson.write(HaraBox.unwrap(value));
              } catch (IllegalArgumentException error) {
                throw new HaraException("json/write: " + error.getMessage());
              }
            }));
    json.define(
        "write-pp",
        new UnaryBuiltin(
            "json/write-pp",
            value -> {
              try {
                return StdJson.writePretty(HaraBox.unwrap(value));
              } catch (IllegalArgumentException error) {
                throw new HaraException("json/write-pp: " + error.getMessage());
              }
            }));
  }

  private void defineFileLibrary() {
    HaraNamespace file = namespace("std.foundation.file");
    file.define("resolve", new VariadicBuiltin("file/resolve", this::fileResolve));
    file.define("read", new UnaryBuiltin("file/read", this::fileRead));
    file.define("write", new VariadicBuiltin("file/write", this::fileWrite));
    file.define("exists?", new UnaryBuiltin("file/exists?", this::fileExists));
    file.define("list", new UnaryBuiltin("file/list", this::fileList));
    file.define("mkdir", new UnaryBuiltin("file/mkdir", this::fileMkdir));
    file.define("delete", new UnaryBuiltin("file/delete", this::fileDelete));
  }

  void installSocketLibrary() {
    withDefinitionOrigin(HaraVar.Origin.JAVA_LIBRARY, this::defineSocketLibrary);
  }

  private void defineSocketLibrary() {
    HaraNamespace socket = namespace("std.foundation.socket");
    socket.define("connect", new VariadicBuiltin("socket/connect", this::socketConnect));
    socket.define("send", new VariadicBuiltin("socket/send", this::socketSend));
    socket.define("close", new UnaryBuiltin("socket/close", this::socketClose));
  }

  private static int int32(Object value, String operation) {
    Object input = HaraBox.unwrap(value);
    if (!(input instanceof Number)) throw new HaraException(operation + " expects integers");
    long number = ((Number) input).longValue();
    if (number < Integer.MIN_VALUE || number > 0xffffffffL) {
      throw new HaraException(operation + " expects a signed 32-bit value");
    }
    return (int) number;
  }

  private static Object bitOperation(String operation, Object[] values) {
    if (values.length != 2) throw new HaraException("bit-" + operation + " expects two integers");
    int left = int32(values[0], "bit-" + operation);
    int right = int32(values[1], "bit-" + operation);
    if ("and".equals(operation)) return (long) (left & right);
    if ("or".equals(operation)) return (long) (left | right);
    return (long) (left ^ right);
  }

  private static Object bitShift(Object[] values, boolean left) {
    String name = left ? "bit-shift-left" : "bit-shift-right";
    if (values.length != 2) throw new HaraException(name + " expects two integers");
    int value = int32(values[0], name);
    int distance = int32(values[1], name);
    if (distance < 0 || distance > 31) {
      throw new HaraException(name + " distance must be in the range 0..31");
    }
    return (long) (left ? value << distance : value >> distance);
  }

  private static String stringValue(Object value, String operation) {
    Object input = HaraBox.unwrap(value);
    if (!(input instanceof String)) throw new HaraException(operation + " expects a string");
    return (String) input;
  }

  private static String[] stringPair(Object[] values, String operation) {
    if (values.length != 2) throw new HaraException(operation + " expects two strings");
    return new String[] {stringValue(values[0], operation), stringValue(values[1], operation)};
  }

  private Object stringCompare(Object[] values) {
    String[] pair = stringPair(values, "str/comp");
    return (long) Integer.signum(pair[0].compareTo(pair[1]));
  }

  private static int codePointLength(String input) {
    return input.codePointCount(0, input.length());
  }

  private Object padString(Object[] values, boolean left) {
    String operation = left ? "str/pad-left" : "str/pad-right";
    if (values.length != 3 || !(HaraBox.unwrap(values[1]) instanceof Number)) {
      throw new HaraException(operation + " expects a string, length, and padding string");
    }
    String input = stringValue(values[0], operation);
    int length = ((Number) HaraBox.unwrap(values[1])).intValue();
    String padding = stringValue(values[2], operation);
    int inputLength = codePointLength(input);
    if (padding.isEmpty() || inputLength >= length) return input;
    int[] paddingCodePoints = padding.codePoints().toArray();
    StringBuilder fill = new StringBuilder();
    for (int index = 0; index < length - inputLength; index++) {
      fill.appendCodePoint(paddingCodePoints[index % paddingCodePoints.length]);
    }
    return left ? fill + input : input + fill;
  }

  private Object stringCharAt(Object[] values) {
    if (values.length != 2 || !(HaraBox.unwrap(values[1]) instanceof Number)) {
      throw new HaraException("str/char-at expects a string and index");
    }
    String input = stringValue(values[0], "str/char-at");
    int index = ((Number) HaraBox.unwrap(values[1])).intValue();
    int length = codePointLength(input);
    if (index < 0 || index >= length) {
      throw new HaraException("str/char-at index out of bounds");
    }
    int charIndex = input.offsetByCodePoints(0, index);
    int codePoint = input.codePointAt(charIndex);
    return new String(Character.toChars(codePoint));
  }

  private Object stringSplit(Object[] values) {
    String[] pair = stringPair(values, "str/split");
    String[] parts = pair[0].split(java.util.regex.Pattern.quote(pair[1]), -1);
    return hara.lang.data.Vector.Standard.from(null, (Object[]) parts);
  }

  private Object stringSplitLines(Object[] values) {
    if (values.length != 1) throw new HaraException("str/split-lines expects one string");
    String input = stringValue(values[0], "str/split-lines");
    String[] parts = input.split("\n", -1);
    return hara.lang.data.Vector.Standard.from(null, (Object[]) parts);
  }

  private Object stringJoin(Object[] values) {
    if (values.length != 2) throw new HaraException("str/join expects a separator and collection");
    String separator = stringValue(values[0], "str/join");
    Iterator<?> iterator = (Iterator<?>) iterValue(values[1]);
    StringBuilder output = new StringBuilder();
    while (iterator.hasNext()) {
      if (output.length() > 0) output.append(separator);
      output.append(stringValue(iterator.next(), "str/join"));
    }
    return output.toString();
  }

  private Object stringIndexOf(Object[] values) {
    if (values.length < 2 || values.length > 3) {
      throw new HaraException("str/index-of expects a string, substring, and optional offset");
    }
    String input = stringValue(values[0], "str/index-of");
    String part = stringValue(values[1], "str/index-of");
    int offset = values.length == 2 ? 0 : ((Number) HaraBox.unwrap(values[2])).intValue();
    int length = codePointLength(input);
    if (offset < 0 || offset > length) return -1L;
    int charOffset = input.offsetByCodePoints(0, offset);
    int result = input.indexOf(part, charOffset);
    return result < 0 ? -1L : (long) input.codePointCount(0, result);
  }

  private Object stringSlice(Object[] values) {
    if (values.length < 2 || values.length > 3) {
      throw new HaraException("str/slice expects a string, start, and optional end");
    }
    String input = stringValue(values[0], "str/slice");
    int start = ((Number) HaraBox.unwrap(values[1])).intValue();
    int end = values.length == 3
        ? ((Number) HaraBox.unwrap(values[2])).intValue()
        : codePointLength(input);
    int length = codePointLength(input);
    if (start < 0 || start > end || end > length) {
      throw new HaraException("str/slice range is out of bounds");
    }
    return input.substring(
        input.offsetByCodePoints(0, start), input.offsetByCodePoints(0, end));
  }

  private Object stringToFixed(Object[] values) {
    if (values.length != 2
        || !(HaraBox.unwrap(values[0]) instanceof Number)
        || !(HaraBox.unwrap(values[1]) instanceof Number)) {
      throw new HaraException("str/to-fixed expects a number and precision");
    }
    int precision = ((Number) HaraBox.unwrap(values[1])).intValue();
    if (precision < 0 || precision > 100) {
      throw new HaraException("str/to-fixed precision must be in the range 0..100");
    }
    return String.format(
        java.util.Locale.ROOT,
        "%." + precision + "f",
        ((Number) HaraBox.unwrap(values[0])).doubleValue());
  }

  private Object stringReplace(Object[] values) {
    if (values.length != 3) {
      throw new HaraException("str/replace expects a string, match, and replacement");
    }
    return stringValue(values[0], "str/replace")
        .replace(stringValue(values[1], "str/replace"), stringValue(values[2], "str/replace"));
  }

  private Object stringReplaceFirst(Object[] values) {
    if (values.length != 3) {
      throw new HaraException("str/replace-first expects a string, match, and replacement");
    }
    String input = stringValue(values[0], "str/replace-first");
    String match = stringValue(values[1], "str/replace-first");
    String replacement = stringValue(values[2], "str/replace-first");
    int index = input.indexOf(match);
    if (index < 0) return input;
    return input.substring(0, index) + replacement + input.substring(index + match.length());
  }

  private Object stringRepeat(Object[] values) {
    if (values.length != 2 || !(HaraBox.unwrap(values[1]) instanceof Number)) {
      throw new HaraException("str/repeat expects a string and count");
    }
    String input = stringValue(values[0], "str/repeat");
    int count = ((Number) HaraBox.unwrap(values[1])).intValue();
    if (count < 0) throw new HaraException("str/repeat count must be non-negative");
    return input.repeat(count);
  }

  private static String stringCapitalize(String input) {
    if (input.isEmpty()) return input;
    int first = input.codePointAt(0);
    int rest = Character.charCount(first);
    return new String(Character.toChars(Character.toUpperCase(first))) + input.substring(rest);
  }

  private static String stringDecapitalize(String input) {
    if (input.isEmpty()) return input;
    int first = input.codePointAt(0);
    int rest = Character.charCount(first);
    return new String(Character.toChars(Character.toLowerCase(first))) + input.substring(rest);
  }

  private Object stringLastIndexOf(Object[] values) {
    if (values.length < 2 || values.length > 3) {
      throw new HaraException("str/last-index-of expects a string, substring, and optional offset");
    }
    String input = stringValue(values[0], "str/last-index-of");
    String part = stringValue(values[1], "str/last-index-of");
    int length = codePointLength(input);
    int offset = values.length == 2
        ? length
        : ((Number) HaraBox.unwrap(values[2])).intValue();
    if (offset < 0) return -1L;
    int charOffset = input.offsetByCodePoints(0, Math.min(offset, length));
    int result = input.lastIndexOf(part, charOffset);
    return result < 0 ? -1L : (long) input.codePointCount(0, result);
  }

  private static byte[] bytesValue(Object value, String operation) {
    Object input = HaraBox.unwrap(value);
    if (!(input instanceof byte[])) throw new HaraException(operation + " expects bytes");
    return (byte[]) input;
  }

  private static int byteNumber(Object value, String operation) {
    Object input = HaraBox.unwrap(value);
    if (!(input instanceof Number)) throw new HaraException(operation + " expects a byte value");
    long number = ((Number) input).longValue();
    if (number < -128 || number > 255) {
      throw new HaraException(operation + " expects a value in the range -128..255");
    }
    return (int) number;
  }

  private Object bytesGet(Object[] values) {
    if (values.length < 2 || values.length > 3) {
      throw new HaraException("bytes/get expects bytes, index, and optional fallback");
    }
    byte[] input = bytesValue(values[0], "bytes/get");
    int index = ((Number) HaraBox.unwrap(values[1])).intValue();
    if (index < 0 || index >= input.length) {
      if (values.length == 3) return values[2];
      throw new HaraException("bytes/get index out of bounds: " + index);
    }
    return (long) Byte.toUnsignedInt(input[index]);
  }

  private Object bytesSet(Object[] values) {
    if (values.length != 3) throw new HaraException("bytes/set expects bytes, index, and value");
    byte[] input = bytesValue(values[0], "bytes/set");
    int index = ((Number) HaraBox.unwrap(values[1])).intValue();
    if (index < 0 || index >= input.length) {
      throw new HaraException("bytes/set index out of bounds: " + index);
    }
    int value = byteNumber(values[2], "bytes/set");
    input[index] = (byte) value;
    return input;
  }

  private Object bytesSlice(Object[] values) {
    if (values.length < 2 || values.length > 3) {
      throw new HaraException("bytes/slice expects bytes, start, and optional end");
    }
    byte[] input = bytesValue(values[0], "bytes/slice");
    int start = ((Number) HaraBox.unwrap(values[1])).intValue();
    int end = values.length == 3 ? ((Number) HaraBox.unwrap(values[2])).intValue() : input.length;
    if (start < 0 || end < start || end > input.length) {
      throw new HaraException("bytes/slice range is out of bounds");
    }
    return java.util.Arrays.copyOfRange(input, start, end);
  }

  private Object promiseRun(Object thunk) {
    CompletableFuture<Object> future =
        CompletableFuture.supplyAsync(
                () -> {
                  try {
                    return invokeInContext(() -> invokeCallable(thunk, new Object[0]));
                  } catch (RuntimeException error) {
                    throw new CompletionException(error);
                  }
                })
            .thenCompose(this::flatten);
    return new HaraPromise(future);
  }

  private Object promiseNew(Object thunk) {
    CompletableFuture<Object> future = new CompletableFuture<>();
    Object resolve =
        new UnaryBuiltin(
            "promise-resolve",
            value -> {
              flatten(value)
                  .whenComplete(
                      (resolved, error) -> {
                        if (error == null) future.complete(resolved);
                        else future.completeExceptionally(error);
                      });
              return value;
            });
    Object reject =
        new UnaryBuiltin(
            "promise-reject",
            value -> {
              future.completeExceptionally(new HaraException(String.valueOf(value)));
              return value;
            });
    try {
      invokeCallable(thunk, new Object[] {resolve, reject});
    } catch (RuntimeException error) {
      future.completeExceptionally(error);
    }
    return new HaraPromise(future);
  }

  private Object promiseFrom(Object value) {
    return new HaraPromise(flatten(value));
  }

  private HaraPromise requirePromise(Object value, String operation) {
    Object input = HaraBox.unwrap(value);
    if (!(input instanceof HaraPromise)) throw new HaraException(operation + " expects a promise");
    return (HaraPromise) input;
  }

  private CompletableFuture<Object> flatten(Object value) {
    Object input = HaraBox.unwrap(value);
    return input instanceof HaraPromise
        ? ((HaraPromise) input).future
        : CompletableFuture.completedFuture(input);
  }

  private Object promiseAll(Object value) {
    ArrayList<CompletableFuture<Object>> promises = new ArrayList<>();
    Iterator<?> iterator = (Iterator<?>) iterValue(value);
    while (iterator.hasNext()) promises.add(flatten(iterator.next()));
    CompletableFuture<?>[] futures = promises.toArray(new CompletableFuture[0]);
    CompletableFuture<Object> result =
        CompletableFuture.allOf(futures)
            .thenApply(
                ignored ->
                    hara.lang.data.Vector.Standard.from(
                        null,
                        promises.stream()
                            .map(promise -> HaraPersistentValues.normalize(promise.join()))
                            .toArray()));
    return new HaraPromise(result);
  }

  private Object promiseThen(Object[] values, boolean failure) {
    String operation = failure ? "promise/catch" : "promise/then";
    if (values.length != 2) throw new HaraException(operation + " expects a promise and function");
    HaraPromise promise = requirePromise(values[0], operation);
    CompletableFuture<Object> result;
    if (failure) {
      result =
          promise
              .future
              .handle(
                  (value, error) ->
                      error == null
                          ? CompletableFuture.completedFuture(value)
                          : flatten(
                              invokeInContext(
                                  () ->
                                      invokeCallable(
                                          values[1],
                                          new Object[] {
                                            error.getCause() == null ? error : error.getCause()
                                          }))))
              .thenCompose(Function.identity());
    } else {
      result =
          promise
              .future
              .thenApply(
                  value ->
                      flatten(
                          invokeInContext(() -> invokeCallable(values[1], new Object[] {value}))))
              .thenCompose(Function.identity());
    }
    return new HaraPromise(result);
  }

  private Object promiseFinally(Object[] values) {
    if (values.length != 2) {
      throw new HaraException("promise/finally expects a promise and function");
    }
    HaraPromise promise = requirePromise(values[0], "promise/finally");
    CompletableFuture<Object> result =
        promise
            .future
            .handle(
                (value, error) ->
                    flatten(invokeInContext(() -> invokeCallable(values[1], new Object[0])))
                        .thenApply(
                            ignored -> {
                              if (error != null) throw new CompletionException(error);
                              return value;
                            }))
            .thenCompose(Function.identity());
    return new HaraPromise(result);
  }

  private Object promiseDelay(Object[] values) {
    if (values.length != 2 || !(HaraBox.unwrap(values[0]) instanceof Number)) {
      throw new HaraException("promise/delay expects milliseconds and a function");
    }
    long millis = ((Number) HaraBox.unwrap(values[0])).longValue();
    if (millis < 0) throw new HaraException("promise/delay expects non-negative milliseconds");
    CompletableFuture<Object> future =
        CompletableFuture.supplyAsync(
                () -> invokeInContext(() -> invokeCallable(values[1], new Object[0])),
                CompletableFuture.delayedExecutor(millis, TimeUnit.MILLISECONDS))
            .thenCompose(this::flatten);
    return new HaraPromise(future);
  }

  <T> T invokeInContext(Supplier<T> operation) {
    Object previous = environment.getContext().enter(null);
    try {
      return operation.get();
    } finally {
      environment.getContext().leave(null, previous);
    }
  }

  private void requireFileIO(String operation) {
    if (!environment.isFileIOAllowed()) {
      throw new HaraException(operation + " is unsupported or file access is denied");
    }
  }

  private void requireSocketIO(String operation) {
    if (!environment.isSocketIOAllowed()) {
      throw new HaraException(operation + " is unsupported or network access is denied");
    }
  }

  private Object socketConnect(Object[] values) {
    requireSocketIO("socket/connect");
    if (values.length != 4 || !(HaraBox.unwrap(values[1]) instanceof Number)) {
      throw new HaraException("socket/connect expects host, port, options, and callback");
    }
    String host = stringValue(values[0], "socket/connect");
    int port = ((Number) HaraBox.unwrap(values[1])).intValue();
    if (port < 1 || port > 65535) throw new HaraException("socket/connect expects a valid port");
    Object callback = values[3];
    CompletableFuture.runAsync(
        () -> {
          try {
            HaraSocket connection = new HaraSocket(new Socket());
            connection.socket.connect(new InetSocketAddress(host, port));
            connection.startDrainer();
            invokeInContext(() -> invokeCallable(callback, new Object[] {null, connection}));
          } catch (Exception error) {
            String message = error.getMessage() == null ? error.getClass().getSimpleName() : error.getMessage();
            invokeInContext(() -> invokeCallable(callback, new Object[] {message, null}));
          }
        });
    return null;
  }

  private Object socketSend(Object[] values) {
    requireSocketIO("socket/send");
    if (values.length != 2) throw new HaraException("socket/send expects a connection and bytes");
    HaraSocket connection = requireSocket(values[0], "socket/send");
    byte[] bytes = bytesValue(values[1], "socket/send");
    synchronized (connection) {
      try {
        connection.socket.getOutputStream().write(bytes);
        connection.socket.getOutputStream().flush();
        return (long) bytes.length;
      } catch (IOException error) {
        throw new HaraException("socket/send failed: " + error.getMessage());
      }
    }
  }

  private Object socketClose(Object value) {
    requireSocketIO("socket/close");
    HaraSocket connection = requireSocket(value, "socket/close");
    try {
      connection.socket.close();
      return null;
    } catch (IOException error) {
      throw new HaraException("socket/close failed: " + error.getMessage());
    }
  }

  private static HaraSocket requireSocket(Object value, String operation) {
    Object input = HaraBox.unwrap(value);
    if (!(input instanceof HaraSocket)) throw new HaraException(operation + " expects a socket connection");
    return (HaraSocket) input;
  }

  private Object fileResolve(Object[] values) {
    requireFileIO("file/resolve");
    if (values.length != 2) throw new HaraException("file/resolve expects a root and path");
    TruffleFile root = environment.getPublicTruffleFile(stringValue(values[0], "file/resolve"));
    return root.resolve(stringValue(values[1], "file/resolve")).normalize().getPath();
  }

  private Object fileRead(Object value) {
    requireFileIO("file/read");
    String path = stringValue(value, "file/read");
    return new HaraPromise(
        CompletableFuture.supplyAsync(
            () ->
                invokeInContext(
                    () -> {
                      try {
                        return environment.getPublicTruffleFile(path).readAllBytes();
                      } catch (IOException error) {
                        throw new CompletionException(error);
                      }
                    })));
  }

  private Object fileWrite(Object[] values) {
    requireFileIO("file/write");
    if (values.length != 2) throw new HaraException("file/write expects a path and bytes");
    String path = stringValue(values[0], "file/write");
    byte[] contents = bytesValue(values[1], "file/write").clone();
    return new HaraPromise(
        CompletableFuture.supplyAsync(
            () ->
                invokeInContext(
                    () -> {
                      try {
                        try (OutputStream output =
                            environment.getPublicTruffleFile(path).newOutputStream()) {
                          output.write(contents);
                        }
                        return null;
                      } catch (IOException error) {
                        throw new CompletionException(error);
                      }
                    })));
  }

  private Object fileExists(Object value) {
    requireFileIO("file/exists?");
    String path = stringValue(value, "file/exists?");
    return new HaraPromise(
        CompletableFuture.supplyAsync(
            () -> invokeInContext(() -> environment.getPublicTruffleFile(path).exists())));
  }

  private Object fileList(Object value) {
    requireFileIO("file/list");
    String path = stringValue(value, "file/list");
    return new HaraPromise(
        CompletableFuture.supplyAsync(
            () ->
                invokeInContext(
                    () -> {
                      try {
                        TruffleFile directory = environment.getPublicTruffleFile(path);
                        java.util.Collection<TruffleFile> children = directory.list();
                        ArrayList<String> names = new ArrayList<>(children.size());
                        for (TruffleFile child : children) {
                          names.add(child.normalize().getPath());
                        }
                        Collections.sort(names);
                        return names.toArray(new String[0]);
                      } catch (IOException error) {
                        throw new CompletionException(error);
                      }
                    })));
  }

  private Object fileMkdir(Object value) {
    requireFileIO("file/mkdir");
    String path = stringValue(value, "file/mkdir");
    return new HaraPromise(
        CompletableFuture.supplyAsync(
            () ->
                invokeInContext(
                    () -> {
                      try {
                        environment.getPublicTruffleFile(path).createDirectories();
                        return null;
                      } catch (IOException error) {
                        throw new CompletionException(error);
                      }
                    })));
  }

  private Object fileDelete(Object value) {
    requireFileIO("file/delete");
    String path = stringValue(value, "file/delete");
    return new HaraPromise(
        CompletableFuture.supplyAsync(
            () ->
                invokeInContext(
                    () -> {
                      try {
                        environment.getPublicTruffleFile(path).delete();
                        return null;
                      } catch (IOException error) {
                        throw new CompletionException(error);
                      }
                    })));
  }

  public Object invokeMarkerMethod(Object receiverValue, String method, Object[] arguments) {
    Object receiver = HaraBox.unwrap(receiverValue);
    if (receiver instanceof HaraArray) {
      return invokeArrayMethod((HaraArray) receiver, method, arguments);
    }
    if (receiver instanceof HaraObject) {
      return invokeObjectMethod((HaraObject) receiver, method, arguments);
    }
    NativeFlavorProvider provider = nativeProvider();
    if (provider == null) {
      throw new HaraException(
          "Dot calls are only supported on values created by array or object unless the namespace selects a native flavor");
    }
    return provider.invokeMember(receiver, method, arguments, nativeAccess());
  }

  private Object invokeArrayMethod(HaraArray array, String method, Object[] arguments) {
    switch (method) {
      case "get":
        requireMethodArity(method, arguments, 1);
        return array.get(arrayIndex(arguments[0], array.size(), false, method));
      case "set":
        requireMethodArity(method, arguments, 2);
        array.set(arrayIndex(arguments[0], array.size(), false, method), arguments[1]);
        return array;
      case "push-last":
        requireMethodArity(method, arguments, 1);
        array.add(arguments[0]);
        return array;
      case "pop-last":
        requireMethodArity(method, arguments, 0);
        if (array.isEmpty()) return null;
        return array.remove(array.size() - 1);
      case "push-first":
        requireMethodArity(method, arguments, 1);
        array.add(0, arguments[0]);
        return array;
      case "pop-first":
        requireMethodArity(method, arguments, 0);
        if (array.isEmpty()) return null;
        return array.remove(0);
      case "insert":
        requireMethodArity(method, arguments, 2);
        array.add(arrayIndex(arguments[0], array.size(), true, method), arguments[1]);
        return array;
      case "remove":
        requireMethodArity(method, arguments, 1);
        return array.remove(arrayIndex(arguments[0], array.size(), false, method));
      case "clone":
        requireMethodArity(method, arguments, 0);
        return new HaraArray(array.toArray());
      case "slice":
        {
          if (arguments.length < 1 || arguments.length > 2) {
            throw new HaraException("array.slice expects a start and optional end");
          }
          int start = arrayIndex(arguments[0], array.size(), true, method);
          int end =
              arguments.length == 2
                  ? arrayIndex(arguments[1], array.size(), true, method)
                  : array.size();
          if (end < start) throw new HaraException("array.slice range is out of bounds");
          return new HaraArray(array.subList(start, end).toArray());
        }
      case "map":
        {
          requireMethodArity(method, arguments, 1);
          HaraArray output = new HaraArray();
          for (Object value : array) output.add(invokeCallable(arguments[0], new Object[] {value}));
          return output;
        }
      case "filter":
        {
          requireMethodArity(method, arguments, 1);
          HaraArray output = new HaraArray();
          for (Object value : array) {
            if (truthy(invokeCallable(arguments[0], new Object[] {value}))) output.add(value);
          }
          return output;
        }
      case "fold-left":
      case "fold-right":
        requireMethodArity(method, arguments, 2);
        Object result = arguments[1];
        if ("fold-left".equals(method)) {
          for (Object value : array) {
            result = invokeCallable(arguments[0], new Object[] {result, value});
          }
        } else {
          for (int i = array.size() - 1; i >= 0; i--) {
            result = invokeCallable(arguments[0], new Object[] {array.get(i), result});
          }
        }
        return result;
      default:
        throw new HaraException("Unsupported array method: " + method);
    }
  }

  private Object invokeObjectMethod(HaraObject object, String method, Object[] arguments) {
    switch (method) {
      case "has?":
        requireMethodArity(method, arguments, 1);
        return object.containsKey(objectKey(arguments[0], method));
      case "get":
        if (arguments.length < 1 || arguments.length > 2) {
          throw new HaraException("object.get expects a key and optional fallback");
        }
        String key = objectKey(arguments[0], method);
        return object.containsKey(key)
            ? object.get(key)
            : arguments.length == 2 ? arguments[1] : null;
      case "set":
        requireMethodArity(method, arguments, 2);
        object.put(objectKey(arguments[0], method), arguments[1]);
        return object;
      case "delete":
        requireMethodArity(method, arguments, 1);
        return object.remove(objectKey(arguments[0], method));
      case "clone":
        requireMethodArity(method, arguments, 0);
        return new HaraObject(object);
      case "assign":
        requireMethodArity(method, arguments, 1);
        Object source = HaraBox.unwrap(arguments[0]);
        if (!(source instanceof HaraObject)) {
          throw new HaraException("object.assign expects an object marker");
        }
        object.putAll((HaraObject) source);
        return object;
      case "keys":
        requireMethodArity(method, arguments, 0);
        return new HaraArray(object.keySet().toArray());
      case "vals":
        requireMethodArity(method, arguments, 0);
        return new HaraArray(object.values().toArray());
      case "pairs":
        {
          requireMethodArity(method, arguments, 0);
          HaraArray pairs = new HaraArray();
          for (Map.Entry<String, Object> entry : object.entrySet()) {
            pairs.add(new HaraArray(new Object[] {entry.getKey(), entry.getValue()}));
          }
          return pairs;
        }
      default:
        throw new HaraException("Unsupported object method: " + method);
    }
  }

  static void requireMethodArity(String method, Object[] arguments, int expected) {
    if (arguments.length != expected) {
      throw new HaraException(method + " expects " + expected + " arguments");
    }
  }

  private static int arrayIndex(Object value, int size, boolean allowEnd, String operation) {
    Object input = HaraBox.unwrap(value);
    if (!(input instanceof Number)) throw new HaraException(operation + " expects a numeric index");
    int index = ((Number) input).intValue();
    if (index < 0 || index > size || (!allowEnd && index == size)) {
      throw new HaraException(operation + " index out of bounds: " + index);
    }
    return index;
  }

  private static String objectKey(Object value, String operation) {
    Object input = HaraBox.unwrap(value);
    if (!(input instanceof String)) {
      throw new HaraException("object." + operation + " expects a string key");
    }
    return (String) input;
  }

  @TruffleBoundary
  private Object arithmetic(String operator, Object[] values) {
    if (operator.equals("+") && values.length == 0) return 0L;
    if (operator.equals("*") && values.length == 0) return 1L;
    if (values.length == 0) {
      throw new HaraException(operator + " expects at least one number");
    }
    if (operator.equals("mod") && values.length != 2) {
      throw new HaraException("mod expects two numbers");
    }
    if (operator.equals("-") && values.length == 1) return Num.minusP(values[0]);
    if (operator.equals("/") && values.length == 1) return Num.divide(1L, values[0]);
    Object result = values[0];
    for (int i = 1; i < values.length; i++) {
      Object value = values[i];
      if (operator.equals("+")) {
        result = Num.addP(result, value);
      } else if (operator.equals("-")) {
        result = Num.minusP(result, value);
      } else if (operator.equals("*")) {
        result = Num.multiplyP(result, value);
      } else if (operator.equals("/")) {
        result = Num.divide(result, value);
      } else if (operator.equals("mod")) {
        result = Num.remainder(result, value);
      } else {
        throw new HaraException("Unknown arithmetic operator: " + operator);
      }
    }
    return result;
  }

  @TruffleBoundary
  private Object compare(String operator, Object[] values) {
    if (values.length < 2) {
      throw new HaraException(operator + " expects at least two arguments");
    }
    Object previous = HaraBox.unwrap(values[0]);
    for (int i = 1; i < values.length; i++) {
      Object current = HaraBox.unwrap(values[i]);
      boolean matches;
      if (operator.equals("=") || operator.equals("not=")) {
        boolean equal = Eq.eq(previous, current);
        if (operator.equals("not=") && !equal) return true;
        matches = equal;
      } else {
        if (!(previous instanceof Number) || !(current instanceof Number)) {
          throw new HaraException("comparison expects two numbers");
        }
        int comparison = Num.compare((Number) previous, (Number) current);
        if (operator.equals("<")) matches = comparison < 0;
        else if (operator.equals("<=")) matches = comparison <= 0;
        else if (operator.equals(">")) matches = comparison > 0;
        else if (operator.equals(">=")) matches = comparison >= 0;
        else throw new HaraException("Unknown comparison operator: " + operator);
      }
      if (!matches) return false;
      previous = current;
    }
    return !operator.equals("not=");
  }

  private Object compareValues(Object[] values) {
    requireMethodArity("compare", values, 2);
    return (long)
        Integer.signum(compareValue(HaraBox.unwrap(values[0]), HaraBox.unwrap(values[1])));
  }

  private int compareValue(Object left, Object right) {
    if (Eq.eq(left, right)) return 0;
    if (left instanceof Number a && right instanceof Number b) return Num.compare(a, b);
    if (left instanceof String a && right instanceof String b) return a.compareTo(b);
    if (left instanceof Character a && right instanceof Character b) return a.compareTo(b);
    if (left instanceof Keyword a && right instanceof Keyword b) return a.compareTo(b);
    if (left instanceof Symbol a && right instanceof Symbol b) {
      return a.display().compareTo(b.display());
    }
    if (left instanceof Boolean a && right instanceof Boolean b) return a.compareTo(b);
    if (left instanceof ILinearType<?> a && right instanceof ILinearType<?> b) {
      Iterator<?> ai = a.iterator();
      Iterator<?> bi = b.iterator();
      while (ai.hasNext() && bi.hasNext()) {
        int result = compareValue(ai.next(), bi.next());
        if (result != 0) return result;
      }
      return Boolean.compare(ai.hasNext(), bi.hasNext());
    }
    throw new HaraException("compare expects two mutually orderable Hara values");
  }

  private Object protocolCall(String protocolName, String methodName, Object[] values) {
    if (values.length == 0) {
      throw new HaraException(methodName + " expects a collection value");
    }
    HaraVar variable = resolve(Symbol.create(protocolName));
    if (variable == null || !(variable.get() instanceof HaraProtocol)) {
      throw new HaraException("Missing protocol: " + protocolName);
    }
    Object receiver = HaraBox.unwrap(values[0]);
    if (isHostObject(receiver)) {
      receiver = asHostObject(receiver);
    }
    Object[] arguments = new Object[values.length - 1];
    System.arraycopy(values, 1, arguments, 0, arguments.length);
    return ((HaraProtocol) variable.get()).invoke(methodName, receiver, arguments);
  }

  private void requireHalPath(String path, String operation) {
    String sourcePath = path.startsWith("classpath:") ? path.substring(10) : path;
    if (!sourcePath.endsWith(".hal") && !sourcePath.endsWith(".hrl")) {
      throw new HaraException(operation + " accepts only .hal or .hrl executable source files");
    }
  }

  private Object loadString(Object value) {
    if (!(value instanceof String)) {
      throw new HaraException("load-string expects a string");
    }
    ContextSnapshot snapshot = snapshot();
    try {
      return parseAndExecute((String) value, "<string>");
    } catch (RuntimeException error) {
      restore(snapshot);
      throw error;
    }
  }

  private Object evalForm(Object value) {
    return parseAndExecute(
        hara.kernel.builtin.BuiltinUtil.prStr(HaraBox.unwrap(value)), "<eval>");
  }

  private Object loadFile(Object value) {
    if (!(value instanceof String)) {
      throw new HaraException("load-file expects a path string");
    }
    requireHalPath((String) value, "load-file");
    requireFileIO("load-file");
    ContextSnapshot snapshot = snapshot();
    try {
      String path = canonicalPath((String) value);
      Object result =
          parseAndExecute(
              new String(
                  environment.getPublicTruffleFile(path).readAllBytes(), StandardCharsets.UTF_8),
              path);
      registerModule(path);
      return result;
    } catch (IOException | RuntimeException error) {
      restore(snapshot);
      throw new HaraException(
          "Unable to load Hara file: " + value + " (" + error.getMessage() + ")");
    }
  }

  @TruffleBoundary
  private Object readForms(Object[] values) {
    requireMethodArity("read-forms", values, 1);
    Object value = HaraBox.unwrap(values[0]);
    if (!(value instanceof String)) {
      throw new HaraException("read-forms expects a path string");
    }
    requireHalPath((String) value, "read-forms");
    requireFileIO("read-forms");
    String path = canonicalPath((String) value);
    try {
      String source =
          new String(
              environment.getPublicTruffleFile(path).readAllBytes(), StandardCharsets.UTF_8);
      return hara.lang.data.Vector.Standard.from(null, HaraLanguage.readAll(source, path));
    } catch (IOException | RuntimeException error) {
      if (error instanceof HaraException) throw (HaraException) error;
      throw new HaraException(
          "Unable to read Hara forms: " + value + " (" + error.getMessage() + ")");
    }
  }

  private String namespaceIdentifier(Object value, String operation) {
    Object unwrapped = unwrapQuoted(HaraBox.unwrap(value));
    if (unwrapped instanceof Symbol symbol) return symbol.display();
    if (unwrapped instanceof String name) return name;
    throw new HaraException(operation + " expects a namespace symbol or string");
  }

  private Object theNamespace(Object value) {
    String name = namespaceIdentifier(value, "the-ns");
    if (!namespaces.containsKey(name)) return null;
    return Symbol.create(name);
  }

  private Object namespaceName(Object value) {
    String name = namespaceIdentifier(value, "ns-name");
    if (!namespaces.containsKey(name)) {
      throw new HaraException("No such namespace: " + name);
    }
    return Symbol.create(name);
  }

  @TruffleBoundary
  private Object namespacePublics(Object value) {
    String name = namespaceIdentifier(value, "ns-publics");
    HaraNamespace target = namespaces.get(name);
    if (target == null) throw new HaraException("No such namespace: " + name);
    ArrayList<Object> entries = new ArrayList<>();
    for (String symbolName : target.sortedSymbolNames()) {
      entries.add(Symbol.create(symbolName));
      entries.add(target.lookup(symbolName));
    }
    return hara.lang.data.OrderedMap.Standard.from(null, entries.toArray());
  }

  @TruffleBoundary
  private Object namespaceAliases(Object value) {
    String name = namespaceIdentifier(value, "ns-aliases");
    if (!namespaces.containsKey(name)) throw new HaraException("No such namespace: " + name);
    ArrayList<Object> entries = new ArrayList<>();
    aliases.getOrDefault(name, Map.of()).entrySet().stream()
        .sorted(Map.Entry.comparingByKey())
        .forEach(
            entry -> {
              entries.add(Symbol.create(entry.getKey()));
              entries.add(Symbol.create(entry.getValue()));
            });
    return hara.lang.data.OrderedMap.Standard.from(null, entries.toArray());
  }

  private Object requiringResolve(Object value) {
    Object unwrapped = unwrapQuoted(HaraBox.unwrap(value));
    if (!(unwrapped instanceof Symbol symbol) || symbol.getNamespace() == null) {
      throw new HaraException("requiring-resolve expects a qualified symbol");
    }
    HaraNamespace target = requiredNamespace(symbol.getNamespace());
    if (target == null) return null;
    return target.lookup(symbol.getName());
  }

  @TruffleBoundary
  private Object loadResource(Object value) {
    if (!(value instanceof String) || ((String) value).isEmpty()) {
      throw new HaraException("load-resource expects a non-empty resource name");
    }
    requireHalPath((String) value, "load-resource");
    ContextSnapshot snapshot = snapshot();
    try {
      FoundationHirLoader.Attempt hir = FoundationHirLoader.load((String) value);
      if (hir.loaded) return hir.value;
    } catch (RuntimeException error) {
      restore(snapshot);
      throw error;
    }
    try (InputStream input =
        HaraContext.class.getClassLoader().getResourceAsStream((String) value)) {
      if (input == null) {
        throw new HaraException("Unable to find Hara resource: " + value);
      }
      return parseAndExecute(
          new String(input.readAllBytes(), StandardCharsets.UTF_8), "classpath:" + value);
    } catch (IOException | RuntimeException error) {
      restore(snapshot);
      if (error instanceof HaraException) throw (HaraException) error;
      throw new HaraException(
          "Unable to load Hara resource: " + value + " (" + error.getMessage() + ")");
    }
  }

  @TruffleBoundary
  public Object requireModule(Object[] arguments) {
    if (arguments.length == 1 || arguments.length == 2) {
      Object requestedNamespace = unwrapQuoted(arguments[0]);
      if (requestedNamespace instanceof Symbol) {
        return requireNamespace((Symbol) requestedNamespace, arguments.length == 2 ? arguments[1] : null);
      }
    }
    if (arguments.length < 1 || arguments.length > 2 || !(arguments[0] instanceof String)) {
      throw new HaraException("require expects a path string or namespace symbol");
    }
    String callerNamespace = currentNamespace.name();
    try {
      String requested = (String) arguments[0];
      requireHalPath(requested, "require");
      boolean classpath = requested.startsWith("classpath:") || getResource(requested) != null;
      String resourceName =
          requested.startsWith("classpath:") ? requested.substring(10) : requested;
      String key = classpath ? "classpath:" + resourceName : canonicalPath(requested);
      boolean reload =
          arguments.length == 2 && requireOption(arguments[1], "reload") == Boolean.TRUE;
      if (!loadingStack.isEmpty()) {
        moduleDependencies
            .computeIfAbsent(loadingStack.peekLast(), ignored -> ConcurrentHashMap.newKeySet())
            .add(key);
      }
      if (reload || !modules.containsKey(key)) {
        if (!loadingModules.add(key)) {
          throw new HaraException("Cyclic module require: " + key);
        }
        Map<String, HaraMacro> callerMacrosBefore =
            new LinkedHashMap<>(macros.getOrDefault(callerNamespace, Map.of()));
        try {
          loadingStack.addLast(key);
          if (classpath) {
            loadResource(resourceName);
            registerResource(resourceName);
          } else {
            loadFile(key);
          }
        } finally {
          loadingStack.removeLastOccurrence(key);
          loadingModules.remove(key);
        }
        ModuleRecord loaded = modules.get(key);
        relocateLoadedMacros(callerNamespace, callerMacrosBefore, loaded);
      }
      currentNamespace = namespace(callerNamespace);
      initializeUserNamespace(currentNamespace);
      if (arguments.length == 2) {
        applyRequireOptions(arguments[1], modules.get(key));
      }
      return null;
    } finally {
      currentNamespace = namespace(callerNamespace);
      initializeUserNamespace(currentNamespace);
    }
  }

  @TruffleBoundary
  private Object requireNamespace(Symbol symbol, Object options) {
    if (symbol.getNamespace() != null) {
      throw new HaraException("require expects an unqualified namespace symbol");
    }
    String target = symbol.display();
    boolean reload = options != null && Boolean.TRUE.equals(requireOption(options, "reload"));
    HaraNamespace required;
    if (reload && libraryLoader.provides(target)) {
      libraryLoader.reload(this, target);
      required = namespaces.get(target);
    } else {
      required = reload ? requireSourceNamespace(target, true) : requiredNamespace(target);
    }
    if (required == null) {
      throw new HaraException("Cannot require missing namespace: " + target);
    }
    if (options != null) applyNamespaceOptions(target, required, options);
    return null;
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private void applyNamespaceOptions(String target, HaraNamespace required, Object options) {
    if (!(options instanceof IMapType<?, ?> map)) throw new HaraException("require options expect a map");
    Object alias = unwrapQuoted(((IMapType) map).lookup(Keyword.create("as")));
    if (alias != null) {
      if (!(alias instanceof Symbol) || ((Symbol) alias).getNamespace() != null) {
        throw new HaraException("require :as expects an unqualified symbol");
      }
      defineAlias((Symbol) alias, Symbol.create(target));
    }
    Object refer = unwrapQuoted(((IMapType) map).lookup(Keyword.create("refer")));
    if (refer != null) {
      java.util.List<Object> symbols = new ArrayList<>();
      if (refer instanceof Keyword keyword && "all".equals(keyword.getName())) {
        symbols.addAll(required.symbolNames());
      } else if (refer instanceof ILinearType<?>) {
        for (Object value : (ILinearType<?>) refer) symbols.add(value);
      } else {
        throw new HaraException("require :refer expects a sequential collection of symbols or :all");
      }
      for (Object value : symbols) {
        if (value instanceof String) value = Symbol.create((String) value);
        if (!(value instanceof Symbol) || ((Symbol) value).getNamespace() != null) {
          throw new HaraException("require :refer expects unqualified symbols");
        }
        String name = ((Symbol) value).getName();
        HaraVar variable = required.lookup(name);
        if (variable == null) throw new HaraException("Cannot refer missing var " + name + " from " + target);
        currentNamespace.refer(name, variable);
        HaraMacro macro = macros.getOrDefault(target, Map.of()).get(name);
        if (macro != null) macros.computeIfAbsent(currentNamespace.name(), ignored -> new ConcurrentHashMap<>()).put(name, macro);
      }
    }
  }

  private void applyRequireOptions(Object options, ModuleRecord module) {
    if (!(options instanceof IMapType<?, ?>) || module == null) {
      throw new HaraException("require options expect a map");
    }
    @SuppressWarnings("rawtypes")
    Object alias = unwrapQuoted(((IMapType) options).lookup(Keyword.create("as")));
    if (alias != null && (!(alias instanceof Symbol) || ((Symbol) alias).getNamespace() != null)) {
      throw new HaraException("require :as expects an unqualified symbol");
    }
    if (alias != null) defineAlias((Symbol) alias, Symbol.create(module.namespace));

    @SuppressWarnings("rawtypes")
    Object refer = unwrapQuoted(((IMapType) options).lookup(Keyword.create("refer")));
    if (refer != null) {
      if (!(refer instanceof ILinearType<?>)) {
        throw new HaraException("require :refer expects a sequential collection of symbols");
      }
      HaraNamespace target = namespaces.get(module.namespace);
      for (Object value : (ILinearType<?>) refer) {
        if (!(value instanceof Symbol) || ((Symbol) value).getNamespace() != null) {
          throw new HaraException("require :refer expects unqualified symbols");
        }
        Symbol symbol = (Symbol) value;
        HaraVar variable = target == null ? null : target.lookup(symbol.getName());
        if (variable == null) {
          throw new HaraException(
              "Cannot refer missing var " + symbol.getName() + " from " + module.namespace);
        }
        currentNamespace.refer(symbol.getName(), variable);
        Map<String, HaraMacro> targetMacros = macros.get(module.namespace);
        if (targetMacros != null && targetMacros.containsKey(symbol.getName())) {
          macros
              .computeIfAbsent(currentNamespace.name(), ignored -> new ConcurrentHashMap<>())
              .put(symbol.getName(), targetMacros.get(symbol.getName()));
        }
      }
    }

    Object referMacros = unwrapQuoted(((IMapType) options).lookup(Keyword.create("refer-macros")));
    if (referMacros == null) return;
    if (!(referMacros instanceof ILinearType<?>)) {
      throw new HaraException("require :refer-macros expects a sequential collection of symbols");
    }
    Map<String, HaraMacro> targetMacros = macros.get(module.namespace);
    for (Object value : (ILinearType<?>) referMacros) {
      if (!(value instanceof Symbol) || ((Symbol) value).getNamespace() != null) {
        throw new HaraException("require :refer-macros expects unqualified symbols");
      }
      String name = ((Symbol) value).getName();
      HaraMacro macro = targetMacros == null ? null : targetMacros.get(name);
      if (macro == null) {
        throw new HaraException("Cannot refer missing macro " + name + " from " + module.namespace);
      }
      macros
          .computeIfAbsent(currentNamespace.name(), ignored -> new ConcurrentHashMap<>())
          .put(name, macro);
    }
  }

  private Object unwrapQuoted(Object value) {
    if (value instanceof List<?>
        && ((List<?>) value).count() == 2
        && Symbol.create("quote").equals(((List<?>) value).nth(0))) {
      return ((List<?>) value).nth(1);
    }
    return value;
  }

  @TruffleBoundary
  private void relocateLoadedMacros(
      String callerNamespace, Map<String, HaraMacro> callerMacrosBefore, ModuleRecord module) {
    if (module == null || callerNamespace.equals(module.namespace)) return;
    Map<String, HaraMacro> callerMacros = macros.get(callerNamespace);
    if (callerMacros == null) return;
    Map<String, HaraMacro> moduleMacros =
        macros.computeIfAbsent(module.namespace, ignored -> new ConcurrentHashMap<>());
    for (Map.Entry<String, HaraMacro> entry : new LinkedHashMap<>(callerMacros).entrySet()) {
      HaraMacro previous = callerMacrosBefore.get(entry.getKey());
      if (previous != entry.getValue()) {
        moduleMacros.put(entry.getKey(), entry.getValue());
        callerMacros.remove(entry.getKey(), entry.getValue());
      }
    }
  }

  @SuppressWarnings("rawtypes")
  private Object requireOption(Object options, String name) {
    if (!(options instanceof IMapType)) {
      throw new HaraException("require options expect a map");
    }
    return ((IMapType) options).lookup(Keyword.create(name));
  }

  private Object moduleRevision(Object value) {
    if (!(value instanceof String)) {
      throw new HaraException("module-revision expects a path string");
    }
    String requested = (String) value;
    String key =
        requested.startsWith("classpath:")
            ? requested
            : (getResource(requested) == null
                ? canonicalPath(requested)
                : "classpath:" + requested);
    ModuleRecord module = modules.get(key);
    return module == null ? 0L : module.revision;
  }

  @TruffleBoundary
  private Object moduleDependencies(Object value) {
    if (!(value instanceof String)) {
      throw new HaraException("module-dependencies expects a path string");
    }
    String requested = (String) value;
    String key =
        requested.startsWith("classpath:")
            ? requested
            : (getResource(requested) == null
                ? canonicalPath(requested)
                : "classpath:" + requested);
    Set<String> dependencies = moduleDependencies.getOrDefault(key, Set.of());
    return BuiltinStruct.vector(new LinkedHashSet<>(dependencies).toArray());
  }

  @TruffleBoundary
  private Object referNamespace(Object value) {
    if (!(value instanceof String)) {
      throw new HaraException("refer expects a namespace string");
    }
    HaraNamespace target = namespaces.get((String) value);
    if (target == null) {
      throw new HaraException("Cannot refer missing namespace: " + value);
    }
    for (Map.Entry<String, HaraVar> entry : target.vars.entrySet()) {
      currentNamespace.refer(entry.getKey(), entry.getValue());
    }
    Map<String, HaraMacro> targetMacros = macros.get(target.name());
    if (targetMacros != null) {
      macros
          .computeIfAbsent(currentNamespace.name(), ignored -> new ConcurrentHashMap<>())
          .putAll(targetMacros);
    }
    return null;
  }

  private Object inNamespace(Object value) {
    if (!(value instanceof Symbol) || ((Symbol) value).getNamespace() != null) {
      throw new HaraException("in-ns expects an unqualified namespace symbol");
    }
    setCurrentNamespace((Symbol) value);
    return value;
  }

  private Object useNamespace(Object value) {
    if (value instanceof Symbol && ((Symbol) value).getNamespace() == null) {
      return referNamespace(((Symbol) value).getName());
    }
    return referNamespace(value);
  }

  @TruffleBoundary
  private Object iterValue(Object value) {
    Object target = HaraBox.unwrap(value);
    if (target == null || target == HaraNull.SINGLETON) return Iter.emptyIterator();
    if (target instanceof String) return Iter.chars(((String) target).toCharArray());
    try {
      return Iter.iter(target);
    } catch (RuntimeException error) {
      throw new HaraException("iter does not support value: " + target);
    }
  }

  @TruffleBoundary
  Object seqValue(Object[] values) {
    if (values.length != 1 && values.length != 2) {
      throw new HaraException("seq expects a source, or a transform and source");
    }
    Object source = values.length == 1 ? values[0] : values[1];
    Object lazySource =
        HaraBox.unwrap(source) instanceof HaraSeq
            ? HaraBox.unwrap(source)
            : new HaraSeq((Iterator<?>) snapshotOrIterator(source));
    if (values.length == 1) {
      return ((HaraSeq) lazySource).hasNext() ? lazySource : null;
    }
    Object result = invokeCallable(values[0], new Object[] {lazySource});
    Object unwrapped = HaraBox.unwrap(result);
    HaraSeq sequence =
        unwrapped instanceof HaraSeq
            ? (HaraSeq) unwrapped
            : new HaraSeq((Iterator<?>) iterValue(unwrapped));
    return sequence.hasNext() ? sequence : null;
  }

  @TruffleBoundary
  private Object isSeq(Object value) {
    return HaraBox.unwrap(value) instanceof HaraSeq;
  }

  @TruffleBoundary
  private Object isIterator(Object value) {
    return HaraBox.unwrap(value) instanceof Iterator<?>;
  }

  private Object snapshotOrIterator(Object value) {
    Object target = HaraBox.unwrap(value);
    if (target instanceof HaraSeq) return target;
    if (target instanceof Iterator<?>) return target;
    if (target instanceof HaraArray) return Iter.objects(((HaraArray) target).toArray());
    return iterValue(target);
  }

  @TruffleBoundary
  private Object iterHasNext(Object value) {
    Iterator<?> iterator = requireIterator(value, "iter-has?");
    return iterator.hasNext();
  }

  @TruffleBoundary
  private Object iterNext(Object value) {
    Iterator<?> iterator = requireIterator(value, "iter-next");
    if (!iterator.hasNext()) throw new HaraException("iter-next reached the end of the iterator");
    return iterator.next();
  }

  private Object iterClose(Object value) {
    Iterator<?> iterator = requireIterator(value, "iter-close");
    Iter.close(iterator);
    return null;
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private Object concatIterators(Object[] values) {
    return Iter.concat(Iter.map((Iterator) Iter.objects(values), value -> Iter.iter(value)));
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private Object iterMap(Object[] values) {
    if (values.length < 2) {
      throw new HaraException("iter-map expects a function and at least one source");
    }
    Object function = values[0];
    Object[] sourceValues = java.util.Arrays.copyOfRange(values, 1, values.length);
    Iterator zipped = iterZipArrays(sourceValues);
    return closeable(
        Iter.map(
            zipped,
            value -> HaraBox.unwrap(invokeCallable(function, (Object[]) value))),
        zipped);
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  Object iterFilter(Object[] values) {
    requireIteratorArity(values, 2, "iter-filter");
    Iterator source = (Iterator) iterValue(values[1]);
    Object function = values[0];
    return closeable(
        Iter.filter(source, value -> truthy(invokeCallable(function, new Object[] {value}))),
        source);
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private Object iterTakeWhile(Object[] values) {
    requireIteratorArity(values, 2, "iter-take-while");
    Iterator source = (Iterator) iterValue(values[1]);
    Object function = values[0];
    return closeable(
        new CloseableIterator<Object>() {
          private boolean finished;
          private boolean ready;
          private Object next;

          private void prime() {
            if (finished || ready) return;
            if (!source.hasNext()) {
              finished = true;
              Iter.close(source);
              return;
            }
            Object candidate = source.next();
            if (!truthy(invokeCallable(function, new Object[] {candidate}))) {
              finished = true;
              Iter.close(source);
              return;
            }
            next = candidate;
            ready = true;
          }

          @Override
          public boolean hasNext() {
            prime();
            return ready;
          }

          @Override
          public Object next() {
            prime();
            if (!ready) throw new NoSuchElementException();
            Object result = next;
            next = null;
            ready = false;
            return result;
          }

          @Override
          public void close() {
            finished = true;
            ready = false;
            next = null;
            Iter.close(source);
          }
        },
        source);
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private Object iterDropWhile(Object[] values) {
    requireIteratorArity(values, 2, "iter-drop-while");
    Iterator source = (Iterator) iterValue(values[1]);
    Object function = values[0];
    return closeable(
        new CloseableIterator<Object>() {
          private boolean dropped;
          private boolean finished;
          private boolean ready;
          private Object next;

          private void prime() {
            if (finished || ready) return;
            while (!dropped && source.hasNext()) {
              Object candidate = source.next();
              if (!truthy(invokeCallable(function, new Object[] {candidate}))) {
                next = candidate;
                ready = true;
                dropped = true;
              }
            }
            if (!dropped) {
              finished = true;
              Iter.close(source);
              return;
            }
            if (!ready && source.hasNext()) {
              next = source.next();
              ready = true;
            } else if (!ready) {
              finished = true;
              Iter.close(source);
            }
          }

          @Override
          public boolean hasNext() {
            prime();
            return ready;
          }

          @Override
          public Object next() {
            prime();
            if (!ready) throw new NoSuchElementException();
            Object result = next;
            next = null;
            ready = false;
            return result;
          }

          @Override
          public void close() {
            finished = true;
            ready = false;
            next = null;
            Iter.close(source);
          }
        },
        source);
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private Object iterMapcat(Object[] values) {
    requireIteratorArity(values, 2, "iter-mapcat");
    Iterator source = (Iterator) iterValue(values[1]);
    Object function = values[0];
    Iterator result =
        Iter.mapcat(
            source, value -> (Iterator) iterValue(invokeCallable(function, new Object[] {value})));
    return closeable(result, source);
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private Object iterKeep(Object[] values) {
    requireIteratorArity(values, 2, "iter-keep");
    Iterator source = (Iterator) iterValue(values[1]);
    Object function = values[0];
    return closeable(
        Iter.keep(
            source,
            value -> {
              Object result = HaraBox.unwrap(invokeCallable(function, new Object[] {value}));
              return result == HaraNull.SINGLETON ? null : result;
            }),
        source);
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private Object iterInterpose(Object[] values) {
    requireIteratorArity(values, 2, "iter-interpose");
    Object separator = values[0];
    Iterator source = (Iterator) iterValue(values[1]);
    return closeable(
        new CloseableIterator<Object>() {
          private boolean first = true;
          private boolean ready;
          private boolean emitSeparator;
          private boolean done;
          private Object next;

          private void prime() {
            if (done || ready) return;
            if (emitSeparator) {
              if (!source.hasNext()) {
                done = true;
                Iter.close(source);
                return;
              }
              next = separator;
              emitSeparator = false;
              ready = true;
              return;
            }
            if (!source.hasNext()) {
              done = true;
              Iter.close(source);
              return;
            }
            next = source.next();
            emitSeparator = source.hasNext();
            first = false;
            ready = true;
          }

          @Override
          public boolean hasNext() {
            prime();
            return ready;
          }

          @Override
          public Object next() {
            prime();
            if (!ready) throw new NoSuchElementException();
            Object result = next;
            next = null;
            ready = false;
            return result;
          }

          @Override
          public void close() {
            done = true;
            ready = false;
            next = null;
            Iter.close(source);
          }
        },
        source);
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private Object iterInterleave(Object[] values) {
    if (values.length == 0) {
      throw new HaraException("iter-interleave expects at least one source");
    }
    Iterator[] sources = new Iterator[values.length];
    for (int i = 0; i < values.length; i++) sources[i] = (Iterator) iterValue(values[i]);
    return new CloseableIterator<Object>() {
      private int index;
      private boolean closed;

      @Override
      public boolean hasNext() {
        if (closed) return false;
        for (Iterator source : sources) {
          if (!source.hasNext()) {
            close();
            return false;
          }
        }
        return true;
      }

      @Override
      public Object next() {
        if (!hasNext()) throw new NoSuchElementException();
        Object result = sources[index].next();
        index = (index + 1) % sources.length;
        return result;
      }

      @Override
      public void close() {
        if (closed) return;
        closed = true;
        for (Iterator source : sources) Iter.close(source);
      }
    };
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private Object iterEvery(Object[] values) {
    requireIteratorArity(values, 2, "iter-every?");
    Iterator source = (Iterator) iterValue(values[1]);
    Object function = values[0];
    return Iter.every(source, value -> truthy(invokeCallable(function, new Object[] {value})));
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private Object iterAny(Object[] values) {
    requireIteratorArity(values, 2, "iter-any?");
    Iterator source = (Iterator) iterValue(values[1]);
    Object function = values[0];
    return Iter.any(source, value -> truthy(invokeCallable(function, new Object[] {value})));
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  Object reduceIterator(Object[] values) {
    if (values.length != 2 && values.length != 3) {
      throw new HaraException(
          "reduce expects a function and collection, optionally with an initial value");
    }
    Object function = values[0];
    Iterator source = (Iterator) iterValue(values[values.length - 1]);
    try {
      Object accumulator;
      if (values.length == 3) {
        accumulator = values[1];
      } else {
        if (!source.hasNext()) {
          throw new HaraException(
              "reduce cannot reduce an empty collection without an initial value");
        }
        accumulator = source.next();
      }
      while (source.hasNext()) {
        accumulator = invokeCallable(function, new Object[] {accumulator, source.next()});
      }
      return accumulator;
    } finally {
      Iter.close(source);
    }
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  Object iterTake(Object[] values) {
    requireIteratorArity(values, 2, "iter-take");
    Iterator source = (Iterator) iterValue(values[1]);
    return closeable(Iter.take(source, iterationCount(values[0], "iter-take")), source);
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  Object iterDrop(Object[] values) {
    requireIteratorArity(values, 2, "iter-drop");
    Iterator source = (Iterator) iterValue(values[1]);
    return closeable(Iter.drop(source, iterationCount(values[0], "iter-drop")), source);
  }

  private Object iterZip(Object[] values) {
    CloseableIterator<Object[]> zipped = iterZipArrays(values);
    return closeable(Iter.map(zipped, BuiltinStruct::vector), zipped);
  }

  private CloseableIterator<Object[]> iterZipArrays(Object[] values) {
    if (values.length == 0) {
      throw new HaraException("iter-zip expects at least one source");
    }
    return new CloseableIterator<Object[]>() {
      private Iterator<?>[] sources;
      private boolean closed;

      private void initialize() {
        if (sources != null) return;
        sources = new Iterator<?>[values.length];
        for (int i = 0; i < values.length; i++) {
          sources[i] = (Iterator<?>) iterValue(values[i]);
        }
      }

      @Override
      public boolean hasNext() {
        if (closed) return false;
        initialize();
        for (Iterator<?> source : sources) {
          if (!source.hasNext()) return false;
        }
        return true;
      }

      @Override
      public Object[] next() {
        if (!hasNext()) throw new NoSuchElementException();
        Object[] result = new Object[sources.length];
        for (int i = 0; i < sources.length; i++) {
          result[i] = sources[i].next();
        }
        return result;
      }

      @Override
      public void close() {
        if (closed) return;
        closed = true;
        if (sources != null) {
          for (Iterator<?> source : sources) Iter.close(source);
        }
      }
    };
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  Object iterCycle(Object value) {
    return Iter.cycle(() -> (Iterator) iterValue(value));
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private Object iterPartitionPair(Object value) {
    Iterator<Object> source = (Iterator<Object>) iterValue(value);
    Iterator<Map.Entry<Object, Object>> pairs = Iter.partitionPair(source);
    return Iter.map(
        pairs, pair -> BuiltinStruct.vector(new Object[] {pair.getKey(), pair.getValue()}));
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private Object iterPartition(Object[] values, boolean includePartial) {
    requireIteratorArity(values, 2, includePartial ? "iter-partition-all" : "iter-partition");
    int size = iterationCount(values[0], includePartial ? "iter-partition-all" : "iter-partition");
    if (size == 0) {
      throw new HaraException(
          (includePartial ? "iter-partition-all" : "iter-partition") + " expects a positive count");
    }
    Iterator source = (Iterator) iterValue(values[1]);
    return closeable(
        new CloseableIterator<Object>() {
          private boolean done;
          private boolean ready;
          private Object next;

          private void prime() {
            if (done || ready) return;
            if (!source.hasNext()) {
              done = true;
              Iter.close(source);
              return;
            }
            ArrayList<Object> chunk = new ArrayList<>(size);
            while (chunk.size() < size && source.hasNext()) {
              chunk.add(source.next());
            }
            if (!includePartial && chunk.size() < size) {
              done = true;
              Iter.close(source);
              return;
            }
            next = BuiltinStruct.vector(chunk.toArray());
            ready = true;
          }

          @Override
          public boolean hasNext() {
            prime();
            return ready;
          }

          @Override
          public Object next() {
            prime();
            if (!ready) throw new NoSuchElementException();
            Object result = next;
            next = null;
            ready = false;
            return result;
          }

          @Override
          public void close() {
            done = true;
            ready = false;
            next = null;
            Iter.close(source);
          }
        },
        source);
  }

  @TruffleBoundary
  private Object iterRange(Object[] values) {
    if (values.length < 1 || values.length > 2) {
      throw new HaraException("iter-range expects an end or start and end");
    }
    if (!(values[0] instanceof Number) || (values.length == 2 && !(values[1] instanceof Number))) {
      throw new HaraException("iter-range expects numeric bounds");
    }
    long start = values.length == 1 ? 0L : ((Number) values[0]).longValue();
    long end = ((Number) values[values.length - 1]).longValue();
    return Iter.range(start, end);
  }

  private Object iterRepeatedly(Object function) {
    return Iter.repeatedly(() -> invokeCallable(function, new Object[0]));
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private Object iterIterate(Object[] values) {
    requireIteratorArity(values, 2, "iter-iterate");
    Object function = values[0];
    return Iter.iterate(values[1], value -> invokeCallable(function, new Object[] {value}));
  }

  private static void requireIteratorArity(Object[] values, int expected, String name) {
    if (values.length != expected) {
      throw new HaraException(name + " expects " + (expected - 1) + " arguments");
    }
  }

  @TruffleBoundary
  private static int iterationCount(Object value, String name) {
    if (!(value instanceof Number)) {
      throw new HaraException(name + " expects a numeric count");
    }
    long count = ((Number) value).longValue();
    if (count < 0 || count > Integer.MAX_VALUE) {
      throw new HaraException(name + " count is out of bounds: " + count);
    }
    return (int) count;
  }

  private static boolean truthy(Object value) {
    return value != null && value != HaraNull.SINGLETON && !Boolean.FALSE.equals(value);
  }

  @SuppressWarnings("unchecked")
  private static Iterator<?> closeable(Iterator<?> iterator, Iterator<?>... sources) {
    return new CloseableIterator<Object>() {
      private boolean closed;

      @Override
      public boolean hasNext() {
        return !closed && iterator.hasNext();
      }

      @Override
      public Object next() {
        if (!hasNext()) throw new NoSuchElementException();
        return iterator.next();
      }

      @Override
      public void close() {
        if (closed) return;
        closed = true;
        for (Iterator<?> source : sources) Iter.close(source);
        Iter.close(iterator);
      }
    };
  }

  private Object alterVarRoot(Object[] values) {
    if (values.length < 2 || !(HaraBox.unwrap(values[0]) instanceof HaraVar)) {
      throw new HaraException("alter-var-root expects a Var, function, and optional arguments");
    }
    HaraVar var = (HaraVar) HaraBox.unwrap(values[0]);
    Object function = HaraBox.unwrap(values[1]);
    Object[] arguments = new Object[values.length - 1];
    arguments[0] = var.deref();
    System.arraycopy(values, 2, arguments, 1, values.length - 2);
    Object updated = invokeCallable(function, arguments);
    return var.reset(updated);
  }

  private Object applyFunction(Object[] values) {
    if (values.length < 2) {
      throw new HaraException("apply expects a function and a final sequential value");
    }
    ArrayList<Object> arguments = new ArrayList<>();
    for (int i = 1; i < values.length - 1; i++) {
      arguments.add(values[i]);
    }
    Iterator<?> tail = (Iterator<?>) iterValue(values[values.length - 1]);
    while (tail.hasNext()) {
      arguments.add(tail.next());
    }
    return invokeCallable(values[0], arguments.toArray());
  }

  @TruffleBoundary
  Object invokeCallable(Object value, Object[] arguments) {
    Object function = HaraBox.unwrap(value);
    if (function instanceof HaraFunction) {
      HaraFunction haraFunction = (HaraFunction) function;
      HaraFunction selected = haraFunction.resolveArity(arguments.length);
      if (selected == null) {
        throw new HaraException("function has no matching arity: " + arguments.length);
      }
      return HaraBox.export(selected.callTarget().call(selected.callArguments(arguments)));
    }
    if (function instanceof HaraMultiFunction) {
      return HaraBox.export(((HaraMultiFunction) function).invoke(arguments));
    }
    if (function instanceof HaraStruct || function instanceof IFn) {
      return ifnProtocol.invoke("invoke", function, arguments);
    }
    if (function instanceof HaraType) {
      HaraType type = (HaraType) function;
      if (arguments.length != type.arity()) {
        throw new HaraException("constructor has no matching arity: " + arguments.length);
      }
      return new HaraStruct(type, arguments);
    }
    throw new HaraException("value is not callable: " + function);
  }

  boolean isFunctionValue(Object value) {
    Object function = HaraBox.unwrap(value);
    return function instanceof HaraFunction
        || function instanceof HaraMultiFunction
        || function instanceof UnaryBuiltin
        || function instanceof VariadicBuiltin;
  }

  private Iterator<?> requireIterator(Object value, String operation) {
    Object target = HaraBox.unwrap(value);
    if (target == null || target == HaraNull.SINGLETON) {
      return Iter.emptyIterator();
    }
    if (!(target instanceof Iterator<?>)) {
      throw new HaraException(operation + " expects an iterator");
    }
    return (Iterator<?>) target;
  }

  private String canonicalPath(String value) {
    return environment.getPublicTruffleFile(value).getAbsoluteFile().normalize().getPath();
  }

  private void registerModule(String path) {
    String key = path;
    ModuleRecord previous = modules.get(key);
    String namespaceName = currentNamespace.name();
    modules.put(
        key, new ModuleRecord(key, namespaceName, previous == null ? 1L : previous.revision + 1L));
    moduleDependencies.computeIfAbsent(key, ignored -> ConcurrentHashMap.newKeySet());
  }

  private void registerResource(String resourceName) {
    String key = "classpath:" + resourceName;
    ModuleRecord previous = modules.get(key);
    String namespaceName = currentNamespace.name();
    modules.put(
        key, new ModuleRecord(key, namespaceName, previous == null ? 1L : previous.revision + 1L));
    moduleDependencies.computeIfAbsent(key, ignored -> ConcurrentHashMap.newKeySet());
  }

  private java.net.URL getResource(String resourceName) {
    return HaraContext.class.getClassLoader().getResource(resourceName);
  }

  @TruffleBoundary
  private ContextSnapshot snapshot() {
    Map<String, Map<String, Object>> values = new LinkedHashMap<>();
    Map<String, Map<String, HaraVar>> bindings = new LinkedHashMap<>();
    Map<String, Map<String, IMetadata>> metadata = new LinkedHashMap<>();
    Map<String, Map<String, HaraVar.Origin>> origins = new LinkedHashMap<>();
    for (Map.Entry<String, HaraNamespace> namespace : namespaces.entrySet()) {
      Map<String, Object> namespaceValues = new LinkedHashMap<>();
      Map<String, HaraVar> namespaceBindings = new LinkedHashMap<>();
      Map<String, IMetadata> namespaceMetadata = new LinkedHashMap<>();
      Map<String, HaraVar.Origin> namespaceOrigins = new LinkedHashMap<>();
      for (Map.Entry<String, HaraVar> var : namespace.getValue().vars.entrySet()) {
        namespaceValues.put(var.getKey(), var.getValue().get());
        namespaceBindings.put(var.getKey(), var.getValue());
        namespaceMetadata.put(var.getKey(), var.getValue().meta());
        namespaceOrigins.put(var.getKey(), var.getValue().origin());
      }
      values.put(namespace.getKey(), namespaceValues);
      bindings.put(namespace.getKey(), namespaceBindings);
      metadata.put(namespace.getKey(), namespaceMetadata);
      origins.put(namespace.getKey(), namespaceOrigins);
    }
    Map<String, Map<String, HaraMacro>> macroValues = new LinkedHashMap<>();
    for (Map.Entry<String, Map<String, HaraMacro>> entry : macros.entrySet()) {
      macroValues.put(entry.getKey(), new LinkedHashMap<>(entry.getValue()));
    }
    Map<String, Map<String, String>> aliasValues = new LinkedHashMap<>();
    for (Map.Entry<String, Map<String, String>> entry : aliases.entrySet()) {
      aliasValues.put(entry.getKey(), new LinkedHashMap<>(entry.getValue()));
    }
    Map<String, Set<String>> dependencyValues = new LinkedHashMap<>();
    for (Map.Entry<String, Set<String>> entry : moduleDependencies.entrySet()) {
      dependencyValues.put(entry.getKey(), new LinkedHashSet<>(entry.getValue()));
    }
    return new ContextSnapshot(
        currentNamespace.name(),
        values,
        bindings,
        metadata,
        origins,
        macroValues,
        aliasValues,
        new LinkedHashMap<>(modules),
        dependencyValues);
  }

  @TruffleBoundary
  private void restore(ContextSnapshot snapshot) {
    namespaces.clear();
    for (Map.Entry<String, Map<String, Object>> entry : snapshot.values.entrySet()) {
      HaraNamespace namespace = namespace(entry.getKey());
      for (Map.Entry<String, Object> value : entry.getValue().entrySet()) {
        HaraVar binding = snapshot.bindings.get(entry.getKey()).get(value.getKey());
        if (binding == null) {
          namespace.define(value.getKey(), value.getValue());
        } else {
          binding.set(value.getValue());
          binding.setMetadata(snapshot.metadata.get(entry.getKey()).get(value.getKey()));
          binding.setOrigin(snapshot.origins.get(entry.getKey()).get(value.getKey()));
          namespace.refer(value.getKey(), binding);
        }
      }
    }
    currentNamespace = namespace(snapshot.currentNamespace);
    macros.clear();
    for (Map.Entry<String, Map<String, HaraMacro>> entry : snapshot.macros.entrySet()) {
      macros.put(entry.getKey(), new ConcurrentHashMap<>(entry.getValue()));
    }
    aliases.clear();
    for (Map.Entry<String, Map<String, String>> entry : snapshot.aliases.entrySet()) {
      aliases.put(entry.getKey(), new ConcurrentHashMap<>(entry.getValue()));
    }
    modules.clear();
    modules.putAll(snapshot.modules);
    moduleDependencies.clear();
    for (Map.Entry<String, Set<String>> entry : snapshot.moduleDependencies.entrySet()) {
      moduleDependencies.put(entry.getKey(), ConcurrentHashMap.newKeySet());
      moduleDependencies.get(entry.getKey()).addAll(entry.getValue());
    }
  }

  private static final class BuiltinExport {
    private final String namespace;
    private final String name;
    private final Object value;
    private final IMetadata metadata;
    private final HaraVar.Origin origin;

    private BuiltinExport(
        String namespace,
        String name,
        Object value,
        IMetadata metadata,
        HaraVar.Origin origin) {
      this.namespace = namespace;
      this.name = name;
      this.value = value;
      this.metadata = metadata == null ? hara.lang.data.Map.Standard.EMPTY : metadata;
      this.origin = origin;
    }
  }

  private static final class ContextSnapshot {
    private final String currentNamespace;
    private final Map<String, Map<String, Object>> values;
    private final Map<String, Map<String, HaraVar>> bindings;
    private final Map<String, Map<String, IMetadata>> metadata;
    private final Map<String, Map<String, HaraVar.Origin>> origins;
    private final Map<String, Map<String, HaraMacro>> macros;
    private final Map<String, Map<String, String>> aliases;
    private final Map<String, ModuleRecord> modules;
    private final Map<String, Set<String>> moduleDependencies;

    private ContextSnapshot(
        String currentNamespace,
        Map<String, Map<String, Object>> values,
        Map<String, Map<String, HaraVar>> bindings,
        Map<String, Map<String, IMetadata>> metadata,
        Map<String, Map<String, HaraVar.Origin>> origins,
        Map<String, Map<String, HaraMacro>> macros,
        Map<String, Map<String, String>> aliases,
        Map<String, ModuleRecord> modules,
        Map<String, Set<String>> moduleDependencies) {
      this.currentNamespace = currentNamespace;
      this.values = values;
      this.bindings = bindings;
      this.metadata = metadata;
      this.origins = origins;
      this.macros = macros;
      this.aliases = aliases;
      this.modules = modules;
      this.moduleDependencies = moduleDependencies;
    }
  }

  private static final class ModuleRecord {
    private final String path;
    private final String namespace;
    private final long revision;

    private ModuleRecord(String path, String namespace, long revision) {
      this.path = path;
      this.namespace = namespace;
      this.revision = revision;
    }
  }

  @TruffleBoundary
  private Object parseAndExecute(String sourceText, String name) {
    try {
      Source source = Source.newBuilder(HaraLanguage.ID, sourceText, name).build();
      return environment.parsePublic(source).call();
    } catch (RuntimeException error) {
      if (error instanceof HaraException) {
        throw (HaraException) error;
      }
      throw new HaraException("Unable to evaluate Hara source " + name + ": " + error.getMessage());
    }
  }

  private static final class HaraArray extends ArrayList<Object> implements ICount, INth<Object> {
    private HaraArray() {}

    private HaraArray(Object[] values) {
      super(java.util.Arrays.asList(values));
    }

    @Override
    public long count() {
      return size();
    }

    @Override
    public Object nth(long index) {
      if (index < 0 || index >= size()) {
        throw new HaraException("nth index out of bounds: " + index);
      }
      return get((int) index);
    }
  }

  private static final class HaraSeq implements CloseableIterator<Object> {
    private final Iterator<?> source;
    private boolean closed;

    private HaraSeq(Iterator<?> source) {
      this.source = source;
    }

    @Override
    public boolean hasNext() {
      return !closed && source.hasNext();
    }

    @Override
    public Object next() {
      if (!hasNext()) throw new NoSuchElementException();
      return source.next();
    }

    @Override
    public void close() {
      if (closed) return;
      closed = true;
      Iter.close(source);
    }

    @Override
    public String toString() {
      return "#<seq>";
    }
  }

  private static final class HaraObject extends LinkedHashMap<String, Object> {
    private HaraObject(Object[] values) {
      if ((values.length & 1) != 0) {
        throw new HaraException("object expects an even number of string key/value arguments");
      }
      for (int i = 0; i < values.length; i += 2) {
        put(objectKey(values[i], "constructor"), values[i + 1]);
      }
    }

    private HaraObject(HaraObject source) {
      super(source);
    }
  }

  private static final class HaraSocket implements IDisplay {
    private final Socket socket;

    private HaraSocket(Socket socket) {
      this.socket = socket;
    }

    private void startDrainer() {
      Thread reader =
          new Thread(
              () -> {
                try (InputStream input = socket.getInputStream()) {
                  byte[] buffer = new byte[8192];
                  while (input.read(buffer) >= 0) {}
                } catch (IOException ignored) {
                  // Closing a socket or a peer disconnecting terminates the background drain.
                }
              },
              "hara-socket-reader");
      reader.setDaemon(true);
      reader.start();
    }


    public String display() {
      return "#<socket " + socket.getRemoteSocketAddress() + ">";
    }
  }

  private static final class HaraPromise implements IDeref<Object>, IDerefTimeout<Object> {
    private final CompletableFuture<Object> future;

    private HaraPromise(CompletableFuture<Object> future) {
      this.future = future;
    }

    private Object state() {
      if (future.isCancelled()) return Keyword.create("cancelled");
      if (!future.isDone()) return Keyword.create("pending");
      return future.isCompletedExceptionally()
          ? Keyword.create("rejected")
          : Keyword.create("fulfilled");
    }

    private Object value() {
      if (!future.isDone()) throw new HaraException("promise is pending");
      return deref();
    }

    private Object cancel() {
      future.cancel(false);
      return this;
    }

    @Override
    public Object deref() {
      try {
        return HaraPersistentValues.normalize(future.join());
      } catch (CompletionException error) {
        Throwable cause = error.getCause() == null ? error : error.getCause();
        if (cause instanceof HaraException) throw (HaraException) cause;
        throw new HaraException("Promise rejected: " + cause.getMessage());
      } catch (java.util.concurrent.CancellationException error) {
        throw new HaraException("Promise cancelled");
      }
    }

    @Override
    public Object derefTimeout(long milliseconds, Object timeoutValue) {
      try {
        return HaraPersistentValues.normalize(future.get(milliseconds, TimeUnit.MILLISECONDS));
      } catch (java.util.concurrent.TimeoutException error) {
        return timeoutValue;
      } catch (InterruptedException error) {
        Thread.currentThread().interrupt();
        throw new HaraException("Promise wait interrupted");
      } catch (java.util.concurrent.ExecutionException error) {
        Throwable cause = error.getCause() == null ? error : error.getCause();
        if (cause instanceof HaraException) throw (HaraException) cause;
        throw new HaraException("Promise rejected: " + cause.getMessage());
      } catch (java.util.concurrent.CancellationException error) {
        throw new HaraException("Promise cancelled");
      }
    }

    @Override
    public String toString() {
      return future.isDone() ? "#<promise realized>" : "#<promise pending>";
    }
  }

  private static final class UnaryBuiltin implements IFn<Object, Object, Object> {
    private final String name;
    private final Function<Object, Object> implementation;

    private UnaryBuiltin(String name, Function<Object, Object> implementation) {
      this.name = name;
      this.implementation = implementation;
    }

    @Override
    public Function<Object, Object> getArg1() {
      return implementation;
    }

    @Override
    public String toString() {
      return "#<builtin " + name + ">";
    }
  }

  private static final class VariadicBuiltin implements IFn<Object, Object, Object> {
    private final String name;
    private final Function<Object[], Object> implementation;

    private VariadicBuiltin(String name, Function<Object[], Object> implementation) {
      this.name = name;
      this.implementation = implementation;
    }

    @Override
    public Supplier<Object> getArg0() {
      return () -> implementation.apply(new Object[0]);
    }

    @Override
    public Function<Object, Object> getArg1() {
      return value -> implementation.apply(new Object[] {value});
    }

    @Override
    public java.util.function.BiFunction<Object, Object, Object> getArg2() {
      return (first, second) -> implementation.apply(new Object[] {first, second});
    }

    @Override
    public Function<Object, Object> getArgN() {
      return values -> implementation.apply((Object[]) values);
    }

    @Override
    public String toString() {
      return "#<builtin " + name + ">";
    }
  }

  private final class HaraNamespace {
    private final String name;
    private final Map<String, HaraVar> vars = new ConcurrentHashMap<>();

    private HaraNamespace(String name) {
      this.name = name;
    }

    private String name() {
      return name;
    }

    private HaraVar lookup(String symbolName) {
      return vars.get(symbolName);
    }

    private java.util.List<String> symbolNames() {
      return new java.util.ArrayList<>(vars.keySet());
    }

    private java.util.List<String> sortedSymbolNames() {
      java.util.ArrayList<String> names = new java.util.ArrayList<>();
      vars.forEach(
          (symbolName, variable) -> {
            if (name.equals(variable.namespaceName())) names.add(symbolName);
          });
      java.util.Collections.sort(names);
      return names;
    }

    @TruffleBoundary
    private HaraVar define(String symbolName, Object value) {
      return define(symbolName, value, null);
    }

    @TruffleBoundary
    private HaraVar define(String symbolName, Object value, IMetadata metadata) {
      return define(symbolName, value, metadata, definitionOrigin);
    }

    @TruffleBoundary
    private HaraVar define(
        String symbolName, Object value, IMetadata metadata, HaraVar.Origin origin) {
      if (collectingBuiltins && name.equals(collectingBuiltinNamespace)) {
        registerBuiltin(name, symbolName, value, metadata, origin);
      }
      return vars.compute(
          symbolName,
          (ignored, existing) -> {
            if (origin == HaraVar.Origin.HAL_FALLBACK
                && existing != null
                && (existing.origin() == HaraVar.Origin.JAVA_LIBRARY
                    || existing.origin() == HaraVar.Origin.RUNTIME_PRIMITIVE)) {
              return existing;
            }
            if (existing == null) {
              return new HaraVar(name, symbolName, value, metadata, origin);
            }
            if (!name.equals(existing.namespaceName())) {
              return new HaraVar(name, symbolName, value, metadata, origin);
            }
            existing.set(value);
            existing.setMetadata(metadata);
            existing.setOrigin(origin);
            return existing;
          });
    }

    private HaraVar refer(String symbolName, HaraVar value) {
      vars.put(symbolName, value);
      return value;
    }
  }
}
