package hara.lang.protocol;

/** Context-aware application and transformation protocol. */
public interface IApplicable {
  Object applyIn(Object runtime, Object[] args);

  default Object applyDefault() {
    return this;
  }

  Object transformIn(Object runtime, Object[] args);

  Object transformOut(Object runtime, Object[] args, Object value);
}
