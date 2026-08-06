package hara.truffle.node;

import com.oracle.truffle.api.frame.VirtualFrame;
import com.oracle.truffle.api.nodes.Node;
import com.oracle.truffle.api.source.SourceSection;

public abstract class HaraExpressionNode extends Node {
  private SourceSection sourceSection;

  public abstract Object execute(VirtualFrame frame);

  public void setHaraSourceSection(SourceSection sourceSection) {
    this.sourceSection = sourceSection;
  }

  @Override
  public SourceSection getSourceSection() {
    return sourceSection;
  }
}
