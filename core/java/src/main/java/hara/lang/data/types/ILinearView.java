package hara.lang.data.types;

public interface ILinearView<E> extends ILinearType<E> {
  public ILinearType<E> subview(int start, int end);
}
