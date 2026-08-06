package hara.lang.data;

import hara.lang.data.types.INamespacedType;
import hara.lang.data.types.IStringType;
import hara.lang.protocol.Constant;
import hara.lang.protocol.IMetadata;

public class Pointer extends INamespacedType.ObjPersistent implements IStringType {

  public Pointer(IMetadata meta, String ns, String name) {
    super(meta, ns, name);
  }

  @Override
  public String display() {
    return "#'" + pathString();
  }

  @Override
  public Pointer withMeta(IMetadata meta) {
    return (_meta == meta) ? this : new Pointer(meta, getNamespace(), getName());
  }

  @Override
  public Constant.ObjType getObjType() {
    return Constant.ObjType.POINTER;
  }
}
