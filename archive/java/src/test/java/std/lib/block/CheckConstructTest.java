package std.lib.block;

import hara.lang.data.Keyword;
import hara.lang.data.Symbol;
import org.junit.Test;

import java.util.List;
import java.util.Map;
import java.util.Set;

import static org.junit.Assert.*;

public class CheckConstructTest {
  @Test
  public void classifiesReferenceInputs() {
    assertTrue(Check.boundary('['));
    assertTrue(Check.boundary('"'));
    assertTrue(Check.whitespace(' '));
    assertTrue(Check.comma(','));
    assertTrue(Check.linebreak('\f'));
    assertTrue(Check.delimiter(')'));
    assertTrue(Check.voidspace('\n'));
    assertTrue(Check.linetab('\t'));
    assertTrue(Check.linespace(' '));
    assertEquals("eof", Check.voidTag(null));
    assertEquals("linetab", Check.voidTag('\t'));
    assertEquals("linebreak", Check.voidTag('\n'));
    assertEquals("comma", Check.voidTag(','));
    assertEquals("delimiter", Check.voidTag(')'));
    assertEquals("symbol", Check.tokenTag(Symbol.create("hello")));
    assertEquals("keyword", Check.tokenTag(Keyword.create("hello")));
    assertEquals("vector", Check.collectionTag(new Object[]{1, 2}));
    assertEquals("map", Check.collectionTag(Map.of()));
    assertEquals("set", Check.collectionTag(Set.of()));
    assertTrue(Check.comment(";hello"));
  }

  @Test
  public void constructsVoidTokensCollectionsAndHelpers() {
    assertSame(Construct.SPACE, Construct.voidBlock());
    assertEquals(4, Construct.tab().width());
    assertEquals(0, Construct.newline().width());
    assertEquals(1, Construct.newline().height());
    assertThrows(IllegalArgumentException.class, () -> Construct.voidBlock('x'));
    assertThrows(IllegalArgumentException.class, () -> Construct.comment("hello"));

    Block.Token string = Construct.token("hello\nworld");
    assertEquals("string", string.tag());
    assertEquals(1, string.height());
    assertTrue(string.verify());

    Block.Container vector = (Block.Container) Construct.block(java.util.Arrays.asList(1L, 2L, 3L));
    assertEquals("(1 2 3)", vector.string());
    assertEquals(7, vector.width());
    assertEquals(5, vector.children().count());

    Block.Container explicit = Construct.container("vector", List.of(Construct.token(1L)));
    assertEquals("[1]", explicit.string());
    assertEquals("[1]", Construct.rep(explicit));
    assertEquals(3, Construct.maxWidth(explicit));
    assertEquals(3, Construct.lineWidth(explicit));
    assertEquals("[1]", Construct.lines(explicit).get(0));
    assertEquals("[12]", Construct.addChild(explicit, 2L).string());
    assertEquals("()", Construct.empty().string());
    assertEquals("1 2", Construct.root(List.of(1L, 2L)).string());
  }
}
