package hara.lang.protocol;

public interface IComponent {

  IMetadata getProps();

  IMetadata getStatus();

  boolean isStarted();

  boolean isStopped();

  IComponent start();

  IComponent stop();

  default IComponent kill() {
    return this.stop();
  }

  default boolean isRemote() {
    return false;
  }
}
