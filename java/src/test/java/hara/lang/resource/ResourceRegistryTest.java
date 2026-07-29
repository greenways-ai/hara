package hara.lang.resource;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertNotNull;
import static org.junit.Assert.assertTrue;

import hara.lang.protocol.IComponent;
import hara.lang.protocol.IMetadata;
import org.junit.Test;

public class ResourceRegistryTest {
  @Test
  public void startsAndStopsScopedComponents() {
    CountingComponent component = new CountingComponent();
    ResourceRegistry registry = new ResourceRegistry();
    registry.add(new ResourceSpec("test", ResourceMode.SHARED, null, options -> component));
    ResourceInstance instance = registry.resolve("test", "default", "one", null);
    assertNotNull(instance);
    assertTrue(instance.isStarted());
    assertEquals(1, component.starts);
    registry.stop("test", "default", "one");
    assertEquals(1, component.stops);
  }

  private static final class CountingComponent implements IComponent {
    int starts;
    int stops;
    boolean started;

    @Override
    public IComponent start() {
      starts++;
      started = true;
      return this;
    }

    @Override
    public IComponent stop() {
      stops++;
      started = false;
      return this;
    }

    @Override
    public IMetadata getProps() {
      return null;
    }

    @Override
    public IMetadata getStatus() {
      return null;
    }

    @Override
    public boolean isStarted() {
      return started;
    }

    @Override
    public boolean isStopped() {
      return !started;
    }
  }
}
