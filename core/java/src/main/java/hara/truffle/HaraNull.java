package hara.truffle;

import com.oracle.truffle.api.interop.InteropLibrary;
import com.oracle.truffle.api.interop.TruffleObject;
import com.oracle.truffle.api.library.ExportLibrary;
import com.oracle.truffle.api.library.ExportMessage;
import hara.lang.protocol.IEquality;

@ExportLibrary(InteropLibrary.class)
final class HaraNull implements TruffleObject, IEquality {
  static final HaraNull SINGLETON = new HaraNull();

  private HaraNull() {}

  @ExportMessage
  boolean isNull() {
    return true;
  }

  @Override
  public boolean equality(Object other) {
    Object unwrapped = HaraBox.unwrap(other);
    return unwrapped == null || unwrapped == SINGLETON;
  }
}
