package hara.lang.protocol;

public interface IContextLifeCycle {
  boolean hasModule(Object moduleId);

  void setupModule(Object moduleId);

  void teardownModule(Object moduleId);

  boolean hasPointer(IPointer pointer);

  void setupPointer(IPointer pointer);

  void teardownPointer(IPointer pointer);
}
