package hara.lang.data;

import hara.lang.data.types.INamespacedType;
import hara.lang.data.types.IStringType;
import hara.lang.protocol.Constant;
import hara.lang.protocol.IMetadata;

import java.lang.ref.WeakReference;
import java.util.Objects;

public class Symbol extends INamespacedType.ObjPersistent implements IStringType {

  public static hara.lang.base.primitive.RefCache<String, Symbol> GLOBAL =
      new hara.lang.base.primitive.RefCache<String, Symbol>();

  public static Symbol create(String ns, String name) {
    String full = (ns == null) ? name : ns + "/" + name;

    return GLOBAL.getOrCreate(
        full,
        () -> {
          var k = new Symbol(null, ns, name);
          return new WeakReference<Symbol>(k, GLOBAL.getQueue());
        });
  }

  public static Symbol create(String nsname) {
    int i = nsname.indexOf('/');
    if (i == -1 || nsname.equals("/")) return create(null, nsname);
    else return create(nsname.substring(0, i), nsname.substring(i + 1));
  }

  public Symbol(IMetadata meta, String nsname) {
    super(meta, nsname);
  }

  public Symbol(IMetadata meta, String ns, String name) {
    super(meta, ns, name);
  }

  /*
  @Override
  public String toString(){
  	return pathString();
  }
  */

  @Override
  public Symbol withMeta(IMetadata meta) {
    return (_meta == meta) ? this : new Symbol(meta, getNamespace(), getName());
  }

  @Override
  public Constant.ObjType getObjType() {
    return Constant.ObjType.SYMBOL;
  }

  @Override
  public String display() {
    return pathString();
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof Symbol symbol
        && Objects.equals(getNamespace(), symbol.getNamespace())
        && getName().equals(symbol.getName());
  }

  @Override
  public int hashCode() {
    return Objects.hash(getNamespace(), getName());
  }
}
