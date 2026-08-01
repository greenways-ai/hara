package hara.truffle.bytecode;

import com.oracle.truffle.api.RootCallTarget;
import com.oracle.truffle.api.CompilerDirectives.TruffleBoundary;
import com.oracle.truffle.api.bytecode.BytecodeConfig;
import com.oracle.truffle.api.bytecode.BytecodeRootNode;
import com.oracle.truffle.api.bytecode.BytecodeRootNodes;
import com.oracle.truffle.api.bytecode.ConstantOperand;
import com.oracle.truffle.api.bytecode.GenerateBytecode;
import com.oracle.truffle.api.bytecode.Operation;
import com.oracle.truffle.api.dsl.Specialization;
import com.oracle.truffle.api.frame.FrameDescriptor;
import com.oracle.truffle.api.nodes.RootNode;
import hara.truffle.HaraContext;
import hara.truffle.HaraLanguage;
import hara.truffle.HbcMachine;

/** Truffle Bytecode DSL entry point for the portable HBC3 instruction set. */
@GenerateBytecode(
    languageClass = HaraLanguage.class,
    enableUncachedInterpreter = true,
    defaultUncachedThreshold = "16",
    enableQuickening = true)
public abstract class HbcBytecodeRootNode extends RootNode implements BytecodeRootNode {
  protected HbcBytecodeRootNode(HaraLanguage language, FrameDescriptor frameDescriptor) {
    super(language, frameDescriptor);
  }

  public static RootCallTarget compile(HaraLanguage language, HbcProgram program) {
    BytecodeRootNodes<HbcBytecodeRootNode> roots =
        HbcBytecodeRootNodeGen.create(
            language,
            BytecodeConfig.DEFAULT,
            builder -> {
              builder.beginRoot();
              builder.beginReturn();
              builder.emitExecute(program);
              builder.endReturn();
              builder.endRoot();
            });
    return roots.getNode(0).getCallTarget();
  }

  @Operation
  @ConstantOperand(type = HbcProgram.class)
  public static final class Execute {
    @Specialization
    @TruffleBoundary
    public static Object execute(HbcProgram program) {
      HaraContext context = HaraLanguage.currentContext();
      return HbcMachine.execute(program, context);
    }
  }
}
