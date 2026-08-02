package hara.truffle.node;

import com.oracle.truffle.api.CompilerDirectives.TruffleBoundary;
import com.oracle.truffle.api.frame.FrameDescriptor;
import com.oracle.truffle.api.frame.MaterializedFrame;
import com.oracle.truffle.api.frame.VirtualFrame;
import com.oracle.truffle.api.nodes.RootNode;
import com.oracle.truffle.api.source.SourceSection;
import hara.kernel.builtin.BuiltinStruct;
import hara.truffle.HaraBox;
import hara.truffle.HaraException;
import hara.truffle.HaraLanguage;
import hara.truffle.EvaluationJournal;

public final class HaraRootNode extends RootNode {
  @Child private HaraExpressionNode body;
  private final int[] parameterSlots;
  private final byte[] parameterKinds;
  private final int[] captureSlots;
  private final int[] captureSourceSlots;
  private final int minimumArity;
  private final boolean variadic;
  private final SourceSection sourceSection;
  private final boolean exportResult;

  public HaraRootNode(
      HaraLanguage language,
      FrameDescriptor descriptor,
      HaraExpressionNode body,
      int[] parameterSlots,
      int[] captureSlots,
      int[] captureSourceSlots,
      SourceSection sourceSection,
      boolean exportResult,
      boolean variadic) {
    this(
        language,
        descriptor,
        body,
        parameterSlots,
        new byte[parameterSlots.length],
        captureSlots,
        captureSourceSlots,
        sourceSection,
        exportResult,
        variadic);
  }

  public HaraRootNode(
      HaraLanguage language,
      FrameDescriptor descriptor,
      HaraExpressionNode body,
      int[] parameterSlots,
      byte[] parameterKinds,
      int[] captureSlots,
      int[] captureSourceSlots,
      SourceSection sourceSection,
      boolean exportResult,
      boolean variadic) {
    super(language, descriptor);
    this.body = body;
    this.parameterSlots = parameterSlots;
    this.parameterKinds = parameterKinds.clone();
    this.captureSlots = captureSlots;
    this.captureSourceSlots = captureSourceSlots;
    this.minimumArity = parameterSlots.length - (variadic ? 1 : 0);
    this.variadic = variadic;
    this.sourceSection = sourceSection;
    this.exportResult = exportResult;
  }

  @Override
  public SourceSection getSourceSection() {
    return sourceSection;
  }

  @Override
  public Object execute(VirtualFrame frame) {
    Object[] arguments = frame.getArguments();
    int argumentOffset = exportResult ? 0 : 1;
    int actualArity = arguments.length - argumentOffset;
    if (actualArity < minimumArity || (!variadic && actualArity != minimumArity)) {
      throw arityError(minimumArity, actualArity, variadic);
    }

    if (!exportResult) {
      MaterializedFrame closure = (MaterializedFrame) arguments[0];
      for (int i = 0; i < captureSlots.length; i++) {
        frame.setObject(captureSlots[i], closure.getValue(captureSourceSlots[i]));
      }
    }
    for (int i = 0; i < minimumArity; i++) {
      Object argument = arguments[i + argumentOffset];
      if (parameterKinds[i] == 1 && argument instanceof Long value) {
        frame.setLong(parameterSlots[i], value);
      } else if (parameterKinds[i] == 2 && argument instanceof Boolean value) {
        frame.setBoolean(parameterSlots[i], value);
      } else {
        frame.setObject(parameterSlots[i], argument);
      }
    }
    if (variadic) {
      Object[] rest = new Object[actualArity - minimumArity];
      System.arraycopy(arguments, minimumArity + argumentOffset, rest, 0, rest.length);
      frame.setObject(parameterSlots[minimumArity], BuiltinStruct.vector(rest));
    }

    long journalOperation = EvaluationJournal.enter(frameLabel(), arguments, argumentOffset);
    try {
      Object result = body.execute(frame);
      EvaluationJournal.returned(journalOperation, result);
      return exportResult ? HaraBox.export(result) : result;
    } catch (RuntimeException error) {
      EvaluationJournal.failed(journalOperation, error);
      if (!HaraException.tracingEnabled()) throw error;
      throw HaraException.withFrame(error, this, frameLabel());
    }
  }

  @TruffleBoundary
  private String frameLabel() {
    if (sourceSection == null || !sourceSection.isAvailable()) return "<hara>";
    return sourceSection.getSource().getName();
  }

  @TruffleBoundary
  private HaraException arityError(int expected, int actual, boolean variadic) {
    String expectedText = variadic ? "at least " + expected : Integer.toString(expected);
    return new HaraException("Expected " + expectedText + " arguments, received " + actual, this);
  }
}
