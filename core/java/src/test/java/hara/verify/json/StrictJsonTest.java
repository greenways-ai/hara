package hara.verify.json;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertNotEquals;
import static org.junit.Assert.assertThrows;

import org.junit.Test;

public class StrictJsonTest {
  @Test
  public void semanticCommitmentsIgnoreObjectOrderAndWhitespace() {
    JsonValue first = StrictJson.parseValue("{\"a\": 1, \"b\": [true, null]}");
    JsonValue second = StrictJson.parseValue(" { \"b\" : [ true , null ], \"a\" : 1 } ");

    assertEquals(JsonCommitment.commit(first), JsonCommitment.commit(second));
    assertNotEquals(
        JsonCommitment.commit(first), JsonCommitment.commit(StrictJson.parseValue("{\"a\":2}")));
  }

  @Test
  public void duplicateKeysAndNonIntegerNumbersFailClosed() {
    assertThrows(IllegalArgumentException.class, () -> StrictJson.parseValue("{\"a\":1,\"a\":2}"));
    assertThrows(IllegalArgumentException.class, () -> StrictJson.parseValue("1.5"));
    assertThrows(IllegalArgumentException.class, () -> StrictJson.parseValue("1e3"));
  }
}
