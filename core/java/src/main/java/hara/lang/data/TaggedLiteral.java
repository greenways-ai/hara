package hara.lang.data;

import hara.lang.base.Eq;
import hara.lang.base.G;
import hara.lang.protocol.IDisplay;
import java.util.Objects;

/** Reader-produced tagged data. It carries no provider authority or live HTA capability. */
public final class TaggedLiteral implements IDisplay {
  private final Symbol tag;
  private final Object form;

  public TaggedLiteral(Symbol tag, Object form) {
    this.tag = Objects.requireNonNull(tag);
    this.form = form;
  }

  public Symbol tag() {
    return tag;
  }

  public Object form() {
    return form;
  }

  @Override
  public String display() {
    return "#" + tag.display() + G.display(form);
  }

  @Override
  public String toString() {
    return display();
  }

  @Override
  public boolean equals(Object other) {
    if (this == other) return true;
    if (!(other instanceof TaggedLiteral)) return false;
    TaggedLiteral tagged = (TaggedLiteral) other;
    return tag.equals(tagged.tag) && Eq.eq(form, tagged.form);
  }

  @Override
  public int hashCode() {
    return 31 * tag.hashCode() + (int) G.hashRapid(form);
  }
}
