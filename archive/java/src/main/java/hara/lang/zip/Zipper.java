package hara.lang.zip;

import hara.lang.data.List;
import hara.lang.protocol.Constant;
import hara.lang.protocol.IMetadata;
import hara.lang.protocol.IObjType;

public class Zipper implements IObjType {
  public final Zipper parent;
  public final List left;
  public final List right;
  public final int depth;
  public final boolean changed;
  public final IZipContext context;
  public final IZipHandler handler;

  public Zipper(
      Zipper parent,
      List left,
      List right,
      int depth,
      boolean changed,
      IZipContext context,
      IZipHandler handler) {
    this.parent = parent;
    this.left = left;
    this.right = right;
    this.depth = depth;
    this.changed = changed;
    this.context = context;
    this.handler = handler;
  }

  public Object currentNode() {
    if (this.right.count() == 0) return null;
    return this.right.peekFirst();
  }

  public Object leftElement() {
    if (this.left.count() == 0) return null;
    return this.left.peekFirst();
  }

  public Object rightElement() {
    if (this.right.count() == 0) return null;
    return this.right.peekFirst();
  }

  public List leftElements() {
    return Zip.reverse(this.left);
  }

  public List rightElements() {
    return this.right;
  }

  public boolean isContainer() {
    return this.context.isContainer(currentNode());
  }

  public boolean isEmptyContainer() {
    return this.context.isEmptyContainer(currentNode());
  }

  public boolean atLeftMost() {
    return this.left.count() == 0;
  }

  public boolean atRightMost() {
    return this.right.count() == 1;
  }

  public boolean atInsideMost() {
    return this.right.count() == 0 || !isContainer();
  }

  public boolean atOutsideMost() {
    return this.parent == null;
  }

  public boolean atInsideMostLeft() {
    return this.left.count() == 0 || !this.context.isContainer(this.left.peekFirst());
  }

  @Override
  public IMetadata meta() {
    return null;
  }

  @Override
  public IObjType withMeta(IMetadata meta) {
    return this;
  }

  @Override
  public long hashCalc(Constant.HashType t) {
    return 0;
  }

  @Override
  public String display() {
    return "#zip" + toString();
  }

  @Override
  public Constant.ObjType getObjType() {
    return Constant.ObjType.CLASS;
  }

  @Override
  public String getObjName() {
    return "ZIP";
  }
}
