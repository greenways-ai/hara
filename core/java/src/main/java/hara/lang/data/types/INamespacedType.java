package hara.lang.data.types;

import hara.lang.protocol.IMetadata;
import hara.lang.protocol.INamespaced;

public interface INamespacedType {

  public abstract class MT extends ObjMutable implements INamespaced {

    private final String _name;
    private final String _ns;
    private transient String _str;

    public MT(IMetadata meta, String nsname) {
      super(meta);

      int i = nsname.indexOf('/');
      if (i == -1 || nsname.equals("/")) {
        _ns = null;
        _name = nsname;
      } else {
        _ns = nsname.substring(0, i);
        _name = nsname.substring(i + 1);
      }
    }

    public MT(IMetadata meta, String ns, String name) {
      super(meta);
      _ns = ns;
      _name = name;
    }

    @Override
    public String getName() {
      return _name;
    }

    @Override
    public String getNamespace() {
      return _ns;
    }

    public String pathString() {
      if (_str == null) {
        if (_ns != null) _str = (_ns + "/" + _name);
        else _str = _name;
      }
      return _str;
    }
  }

  public abstract class ObjPersistent extends hara.lang.data.types.ObjPersistent
      implements INamespaced {

    private final String _name;
    private final String _ns;
    private transient String _str;

    public ObjPersistent(IMetadata meta, String nsname) {
      super(meta);

      int i = nsname.indexOf('/');
      if (i == -1 || nsname.equals("/")) {
        _ns = null;
        _name = nsname;
      } else {
        _ns = nsname.substring(0, i);
        _name = nsname.substring(i + 1);
      }
    }

    public ObjPersistent(IMetadata meta, String ns, String name) {
      super(meta);
      _ns = ns;
      _name = name;
    }

    @Override
    public String getName() {
      return _name;
    }

    @Override
    public String getNamespace() {
      return _ns;
    }

    public String pathString() {
      if (_str == null) {
        if (_ns != null) _str = (_ns + "/" + _name);
        else _str = _name;
      }
      return _str;
    }

    @Override
    public String display() {
      return pathString();
    }
  }
}
