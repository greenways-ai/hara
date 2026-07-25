package hara.truffle;

import org.junit.Test;
import org.junit.runner.JUnitCore;
import org.junit.runner.RunWith;
import static org.junit.Assert.assertTrue;

@RunWith(HaraTestRunner.class)
@HaraTestResources("hara/test-facts.hal")
class HaraRunnerFixture {}

public class HaraTestRunnerTest {
  @Test
  public void executesHaraFactsThroughJUnit() {
    assertTrue(new JUnitCore().run(HaraRunnerFixture.class).wasSuccessful());
  }
}
