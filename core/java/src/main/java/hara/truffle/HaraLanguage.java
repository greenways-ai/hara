package hara.truffle;

import com.oracle.truffle.api.CallTarget;
import com.oracle.truffle.api.TruffleLanguage;
import com.oracle.truffle.api.nodes.Node;
import com.oracle.truffle.api.source.Source;
import com.oracle.truffle.api.source.SourceSection;
import hara.kernel.base.Parser;
import hara.kernel.base.Reader;
import hara.truffle.bytecode.HbcBytecodeRootNode;
import hara.truffle.bytecode.HbcCodec;
import hara.lang.data.Keyword;
import hara.lang.data.Map;
import hara.lang.protocol.IMetadata;
import hara.lang.protocol.IObjType;
import java.util.ArrayList;
import java.util.List;

@TruffleLanguage.Registration(
    id = HaraLanguage.ID,
    name = "Hara",
    implementationName = "Hara Truffle",
    version = "0.1",
    defaultMimeType = HaraLanguage.MIME_TYPE,
    characterMimeTypes = HaraLanguage.MIME_TYPE,
    byteMimeTypes = HaraLanguage.BYTECODE_MIME_TYPE)
public final class HaraLanguage extends TruffleLanguage<HaraContext> {
  public static final String ID = "hara";
  public static final String MIME_TYPE = "application/x-hara";
  public static final String BYTECODE_MIME_TYPE = "application/x-hara-bytecode";

  private static final ContextReference<HaraContext> CONTEXT_REFERENCE =
      ContextReference.create(HaraLanguage.class);

  @Override
  protected HaraContext createContext(Env environment) {
    return new HaraContext(environment);
  }

  @Override
  protected void finalizeContext(HaraContext context) {
    context.closeExtensions();
  }

  public static HaraContext currentContext() {
    return getCurrentContext(HaraLanguage.class);
  }

  /**
   * Node-local context lookup for hot execution paths: resolves through the node's root and
   * engine caches instead of walking the stack the way {@link #currentContext()} does.
   */
  public static HaraContext currentContext(Node node) {
    return CONTEXT_REFERENCE.get(node);
  }

  static HaraLanguage currentLanguage() {
    return getCurrentLanguage(HaraLanguage.class);
  }

  @Override
  protected boolean isThreadAccessAllowed(Thread thread, boolean singleThreaded) {
    return true;
  }

  @Override
  protected CallTarget parse(ParsingRequest request) {
    currentContext().ensureEagerFallbacks();
    Source source = request.getSource();
    if (source.hasBytes() || BYTECODE_MIME_TYPE.equals(source.getMimeType())) {
      try {
        return HbcBytecodeRootNode.compile(this, HbcCodec.decode(source.getBytes().toByteArray()));
      } catch (RuntimeException error) {
        throw new HaraException("Unable to read Hara bytecode " + source.getName() + ": " + error.getMessage());
      }
    }
    SourceSection sourceSection =
        source.getLength() == 0
            ? source.createUnavailableSection()
            : source.createSection(0, source.getLength());
    Object[] forms;
    try {
      forms = readAll(source.getCharacters().toString(), source.getName());
    } catch (hara.kernel.base.Parser.LispReader.ReaderException error) {
      Throwable cause = error.getCause();
      String detail = cause == null ? error.getMessage() : cause.getMessage();
      throw new HaraException(
          "Unable to read Hara source "
              + source.getName()
              + " at line "
              + error.line()
              + ", column "
              + error.column()
              + ": "
              + detail);
    }
    return HaraAnalyzer.compile(this, forms, sourceSection, currentContext());
  }

  static CallTarget compileHalc(Object[] forms, String sourceName) {
    return FoundationHalcLowerer.compile(currentLanguage(), currentContext(), forms);
  }

  static CallTarget compileHalc(HalcArtifact.Module module, String sourceName) {
    HaraContext context = currentContext();
    context.installHalcSchemas(module.schemas);
    return FoundationHalcLowerer.compile(currentLanguage(), context, module.forms);
  }

  static Object[] readAll(String source, String sourceName) {
    Reader reader = new Reader(source);
    Object eof = new Object();
    List<Object> forms = new ArrayList<>();
    Object form;
    do {
      form = Parser.LispReader.read(reader, false, eof, false, null);
      if (form != eof) {
        forms.add(withSourceName(form, sourceName));
      }
    } while (form != eof);
    return forms.toArray();
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private static Object withSourceName(Object form, String sourceName) {
    if (!(form instanceof IObjType) || sourceName == null) return form;
    IObjType object = (IObjType) form;
    IMetadata metadata = object.meta();
    hara.lang.data.types.IMapType updated =
        metadata instanceof hara.lang.data.types.IMapType
            ? (hara.lang.data.types.IMapType)
                ((hara.lang.data.types.IMapType) metadata)
                    .assoc(Keyword.create("file"), sourceName)
            : Map.Standard.from(null, Keyword.create("file"), sourceName);
    return object.withMeta(updated);
  }
}
