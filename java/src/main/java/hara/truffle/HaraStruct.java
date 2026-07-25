package hara.truffle;

import com.oracle.truffle.api.CompilerDirectives.TruffleBoundary;
import com.oracle.truffle.api.interop.InteropLibrary;
import com.oracle.truffle.api.interop.InvalidArrayIndexException;
import com.oracle.truffle.api.interop.TruffleObject;
import com.oracle.truffle.api.interop.UnknownIdentifierException;
import com.oracle.truffle.api.library.ExportLibrary;
import com.oracle.truffle.api.library.ExportMessage;
import java.util.Arrays;
import java.util.Iterator;
import java.util.Map;
import hara.lang.data.Keyword;
import hara.lang.data.Symbol;
import hara.lang.data.Tuple;
import hara.lang.protocol.Constant;
import hara.lang.protocol.IAssoc;
import hara.lang.protocol.ICount;
import hara.lang.protocol.IDissoc;
import hara.lang.protocol.IEmpty;
import hara.lang.protocol.IFind;
import hara.lang.protocol.ILookup;
import hara.lang.protocol.IMetadata;
import hara.lang.protocol.IObjType;

@ExportLibrary(InteropLibrary.class)
public final class HaraStruct
    implements TruffleObject,
        IObjType,
        ILookup<Object, Object>,
        IAssoc<Object, Object>,
        IFind<Object, Map.Entry<Object, Object>>,
        IDissoc<Object>,
        IEmpty,
        ICount,
        Iterable<Map.Entry<Object, Object>> {
  private final HaraType type;
  private final Object[] values;
  private final IMetadata metadata;

  public HaraStruct(HaraType type, Object[] values) {
    this(type, values, null);
  }

  private HaraStruct(HaraType type, Object[] values, IMetadata metadata) {
    this.type = type;
    this.values = values.clone();
    this.metadata = metadata;
  }

  public Object read(String field) throws UnknownIdentifierException {
    int index = type.fieldIndex(field);
    if (index < 0) {
      throw UnknownIdentifierException.create(field);
    }
    return values[index];
  }

  public HaraType type() {
    return type;
  }

  @Override
  public IMetadata meta() {
    return metadata;
  }

  @Override
  public HaraStruct withMeta(IMetadata metadata) {
    return new HaraStruct(type, values, metadata);
  }

  @Override
  public long hashCalc(Constant.HashType hashType) {
    return hashCode();
  }

  @Override
  public String display() {
    return toString();
  }

  @ExportMessage
  boolean hasMembers() {
    return true;
  }

  @ExportMessage
  Object getMembers(boolean includeInternal) {
    return new HaraMemberNames(type.fields());
  }

  @ExportMessage
  boolean isMemberReadable(String member) {
    return type.fieldIndex(member) >= 0;
  }

  @ExportMessage
  Object readMember(String member) throws UnknownIdentifierException {
    return HaraBox.export(read(member));
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof HaraStruct
        && type == ((HaraStruct) other).type
        && Arrays.deepEquals(values, ((HaraStruct) other).values);
  }

  @Override
  public int hashCode() {
    return 31 * System.identityHashCode(type) + Arrays.deepHashCode(values);
  }

  @Override
  @TruffleBoundary
  public String toString() {
    StringBuilder result = new StringBuilder("#<").append(type.name());
    for (int i = 0; i < values.length; i++) {
      result.append(i == 0 ? " " : ", ").append(type.fields()[i]).append("=").append(values[i]);
    }
    return result.append(">").toString();
  }

  @Override
  public Map.Entry<Object, Object> find(Object key) {
    int index = indexOfKey(key);
    return index < 0 ? null : new Tuple.Tup2.L<>(null, key, values[index]);
  }

  @Override
  public Iterator<Object> keys() {
    String[] fields = type.fields();
    return new Iterator<Object>() {
      private int index = 0;

      @Override
      public boolean hasNext() {
        return index < fields.length;
      }

      @Override
      public Object next() {
        return Keyword.create(fields[index++]);
      }
    };
  }

  @Override
  public Iterator<Object> vals() {
    return Arrays.asList(values).iterator();
  }

  @Override
  public Iterator<Map.Entry<Object, Object>> iterator() {
    String[] fields = type.fields();
    return new Iterator<Map.Entry<Object, Object>>() {
      private int index = 0;

      @Override
      public boolean hasNext() {
        return index < values.length;
      }

      @Override
      public Map.Entry<Object, Object> next() {
        Object key = Keyword.create(fields[index]);
        Object value = values[index];
        index++;
        return new Tuple.Tup2.L<>(null, key, value);
      }
    };
  }

  @Override
  public IAssoc<Object, Object> assoc(Object key, Object value) {
    int index = indexOfKey(key);
    if (index < 0) {
      return asMap().assoc(key, value);
    }
    Object[] newValues = values.clone();
    newValues[index] = value;
    return new HaraStruct(type, newValues, metadata);
  }

  @Override
  public IDissoc<Object> dissoc(Object key) {
    int index = indexOfKey(key);
    if (index < 0) {
      return this;
    }
    Object[] elements = new Object[(values.length - 1) * 2];
    int position = 0;
    String[] fields = type.fields();
    for (int i = 0; i < values.length; i++) {
      if (i == index) {
        continue;
      }
      elements[position++] = Keyword.create(fields[i]);
      elements[position++] = values[i];
    }
    return hara.lang.data.Map.Standard.<Object, Object>from(metadata, elements);
  }

  @Override
  public IEmpty empty() {
    return new HaraStruct(type, new Object[values.length], metadata);
  }

  @Override
  public long count() {
    return values.length;
  }

  private hara.lang.data.Map.Standard<Object, Object> asMap() {
    String[] fields = type.fields();
    Object[] elements = new Object[values.length * 2];
    for (int i = 0; i < values.length; i++) {
      elements[i * 2] = Keyword.create(fields[i]);
      elements[i * 2 + 1] = values[i];
    }
    return hara.lang.data.Map.Standard.from(metadata, elements);
  }

  private int indexOfKey(Object key) {
    if (key instanceof Keyword) {
      Keyword keyword = (Keyword) key;
      if (keyword.getNamespace() != null) {
        return -1;
      }
      return type.fieldIndex(keyword.getName());
    }
    if (key instanceof Symbol) {
      Symbol symbol = (Symbol) key;
      if (symbol.getNamespace() != null) {
        return -1;
      }
      return type.fieldIndex(symbol.getName());
    }
    if (key instanceof String) {
      return type.fieldIndex((String) key);
    }
    return -1;
  }

  @ExportLibrary(InteropLibrary.class)
  static final class HaraMemberNames implements TruffleObject {
    private final String[] names;

    HaraMemberNames(String[] names) {
      this.names = names;
    }

    @ExportMessage
    boolean hasArrayElements() {
      return true;
    }

    @ExportMessage
    long getArraySize() {
      return names.length;
    }

    @ExportMessage
    boolean isArrayElementReadable(long index) {
      return index >= 0 && index < names.length;
    }

    @ExportMessage
    Object readArrayElement(long index) throws InvalidArrayIndexException {
      if (!isArrayElementReadable(index)) {
        throw InvalidArrayIndexException.create(index);
      }
      return names[(int) index];
    }
  }
}
