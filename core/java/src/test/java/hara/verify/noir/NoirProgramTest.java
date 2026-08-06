package hara.verify.noir;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertNotEquals;
import static org.junit.Assert.assertThrows;

import org.junit.Test;

public class NoirProgramTest {
  private static final String SOURCE = "fn main(value: u8) { assert(value > 0); }";

  @Test
  public void cacheKeyMatchesTheBrowserLoaderVector() {
    NoirProgram program = NoirProgram.create("first_cut", SOURCE);

    assertEquals(
        "6db358723409593229a47cd5271d5073dd67f38c55bc0a643857afb2206e8c5f",
        program.cacheKey().hex());
  }

  @Test
  public void toolchainVersionsArePartOfProgramIdentity() {
    NoirProgram first = NoirProgram.create("first_cut", SOURCE, "1.0.0-beta.25", "unbound");
    NoirProgram otherCompiler = NoirProgram.create("first_cut", SOURCE, "1.0.0-beta.24", "unbound");
    NoirProgram otherBackend = NoirProgram.create("first_cut", SOURCE, "1.0.0-beta.25", "bb/other");

    assertNotEquals(first.cacheKey(), otherCompiler.cacheKey());
    assertNotEquals(first.cacheKey(), otherBackend.cacheKey());
  }

  @Test
  public void invalidPackageNamesFailBeforeTheLoaderRuns() {
    assertThrows(IllegalArgumentException.class, () -> NoirProgram.create("Not Safe", SOURCE));
  }
}
