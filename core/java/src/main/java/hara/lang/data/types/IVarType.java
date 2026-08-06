package hara.lang.data.types;

import hara.lang.protocol.IDeref;

public interface IVarType extends IDeref<Object> {
  Boolean isDynamic();

  Boolean isMacro();

  Boolean isControl();
}
