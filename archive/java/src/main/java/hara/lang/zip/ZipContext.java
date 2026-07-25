package hara.lang.zip;

import hara.lang.data.List;
import hara.lang.data.Vector;
import hara.lang.protocol.IColl;

public class ZipContext implements IZipContext {

  @Override
  public Object createContainer() {
    return Vector.Standard.empty(null);
  }

  @Override
  public Object createElement(Object data) {
    return data;
  }

  @Override
  public boolean isContainer(Object element) {
    return element instanceof IColl;
  }

  @Override
  public boolean isEmptyContainer(Object element) {
    if (element instanceof IColl) {
      return ((IColl<?>) element).count() == 0;
    }
    return false;
  }

  @Override
  public boolean isElement(Object element) {
    return element != null;
  }

  @Override
  public List listElements(Object container) {
    if (isContainer(container)) {
      return hara.lang.data.List.Standard.into(((IColl) container).iterator());
    }
    // This should not be reached if isContainer is checked before calling
    throw new IllegalArgumentException("Cannot list elements of a non-container type.");
  }

  @Override
  public Object updateElements(Object container, List newElements) {
    if (container instanceof Vector.Base) {
      Vector.Mutable mut = Vector.Mutable.empty(null);
      for (Object o : newElements) {
        mut = (Vector.Mutable) mut.conj(o);
      }
      return mut.toPersistent();
    } else if (container instanceof hara.lang.data.List.Base) {
      return hara.lang.data.List.Standard.into(newElements.iterator());
    }
    // Fallback or throw exception for other container types
    throw new UnsupportedOperationException(
        "updateElements not implemented for this container type: " + container.getClass());
  }

  @Override
  public Object addElement(Object container, Object element) {
    if (isContainer(container)) {
      return ((IColl) container).conj(element);
    }
    throw new IllegalArgumentException("Cannot add element to a non-container type.");
  }

  @Override
  public Object wrapData(Object data) {
    return data; // No-op for now
  }

  @Override
  public Object unwrapData(Object data) {
    return data; // No-op for now
  }
}
