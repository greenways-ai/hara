package hara.lang.protocol;

/** A resumable computation with an explicit lifecycle. */
public interface ICoroutine extends IClose {
  Object status();

  Object resume(Object... arguments);
}
