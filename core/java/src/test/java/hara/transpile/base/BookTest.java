package hara.transpile.base;

import hara.lang.data.Keyword;
import org.junit.Test;
import static org.junit.Assert.*;
import java.util.Collections;
import java.util.Map;

public class BookTest {

  @Test
  public void testBookConstruction() {
    Keyword lang = Keyword.create("java");
    Map meta = Collections.emptyMap();
    Grammar grammar = new Grammar();
    Map modules = Collections.emptyMap();
    Keyword parent = Keyword.create("base");

    Book book = new Book(lang, meta, grammar, modules, parent);

    assertEquals(lang, book.lang);
    assertEquals(meta, book.meta);
    assertEquals(grammar, book.grammar);
    assertEquals(modules, book.modules);
    assertEquals(parent, book.parent);
  }
}
