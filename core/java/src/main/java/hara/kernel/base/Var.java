package hara.kernel.base;

import hara.lang.data.Map;
import hara.lang.data.types.INamespacedType;
import hara.lang.data.types.IStringType;
import hara.lang.data.types.IVarType;
import hara.lang.protocol.Constant;
import hara.lang.protocol.IReset;

import static hara.kernel.builtin.BuiltinBasic.keyword;

public class Var extends INamespacedType.MT implements IStringType, IVarType, IReset<Object> {

  Object _val;

  public Var(String nsname, Object v) {
    super(Map.Standard.EMPTY, nsname);
    _val = v;
  }

  @Override
  public Object deref() {
    return _val;
  }

  @Override
  public String display() {
    return "#'" + pathString();
  }

  @Override
  public Constant.ObjType getObjType() {
    return Constant.ObjType.POINTER;
  }

  @Override
  public Boolean isControl() {
    return (Boolean) keyword("control").invoke(_meta, false);
  }

  @Override
  public Boolean isDynamic() {
    return (Boolean) keyword("dynamic").invoke(_meta, false);
  }

  @Override
  public Boolean isMacro() {
    return (Boolean) keyword("macro").invoke(_meta, false);
  }

  @Override
  public Object reset(Object v) {
    return _val = v;
  }
}
