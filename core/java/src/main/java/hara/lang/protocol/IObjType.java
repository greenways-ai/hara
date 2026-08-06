package hara.lang.protocol;

public interface IObjType extends IHash, IDisplay {

  default Constant.ObjType getObjType() {
    return Constant.ObjType.CLASS;
  }

  default String getObjName() {
    return getObjType().toString();
  }

  @Override
  default String hashSeed() {
    return "::" + getObjName() + "";
  }

  IMetadata meta();

  IObjType withMeta(IMetadata meta);
}
