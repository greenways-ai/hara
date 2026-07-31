package hara.truffle;

import static org.junit.Assert.assertEquals;

import org.junit.Test;

public class HaraIdentityToolTest {
  @Test
  public void enrollmentBytesMatchTheRustContract() {
    assertEquals(
        "{:enrollment/format 1 :enrollment/tap \"hara\" "
            + ":enrollment/provider :github :enrollment/owner \"alice\" "
            + ":enrollment/public-key \""
            + "ab".repeat(32)
            + "\" :enrollment/challenge \"challenge-1\"}\n",
        HaraIdentityTool.canonicalEnrollment(
            "hara", "alice", "ab".repeat(32), "challenge-1"));
  }
}
