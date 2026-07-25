package hara.lang.zip;

import hara.lang.base.Eq;
import hara.lang.data.List;
import hara.lang.data.Vector;
import hara.lang.protocol.Constant;
import org.junit.Test;

import java.util.ArrayList;

import static org.junit.Assert.*;

public class ZipComprehensiveTest {
  @Test
  public void zipperStatePredicatesAndProtocolMethodsAreConsistent() {
    Vector<Object> root = Vector.Standard.from(null, 1, 2);
    Zipper zipper = Zip.zipper(root);
    assertEquals(0, zipper.depth);
    assertFalse(zipper.changed);
    assertTrue(zipper.atLeftMost());
    assertTrue(zipper.atRightMost());
    assertTrue(zipper.atOutsideMost());
    assertFalse(zipper.atInsideMost());
    assertTrue(zipper.isContainer());
    assertFalse(zipper.isEmptyContainer());
    assertNull(zipper.leftElement());
    assertEquals(root, zipper.rightElement());
    assertEquals(Constant.ObjType.CLASS, zipper.getObjType());
    assertEquals("ZIP", zipper.getObjName());
    assertTrue(zipper.display().startsWith("#zip"));
    assertNull(zipper.meta());
    assertSame(zipper, zipper.withMeta(null));
    assertEquals(0, zipper.hashCalc(null));
  }

  @Test
  public void navigationTracksDepthLeftAndRightElements() {
    Vector<Object> root = Vector.Standard.from(null, 1, Vector.Standard.from(null, 2, 3), 4);
    Zipper zipper = Zip.stepInside(Zip.zipper(root));
    assertEquals(1, zipper.depth);
    assertEquals(1, zipper.currentNode());
    assertTrue(zipper.atLeftMost());
    assertTrue(Eq.eq(List.Standard.from(null, 1, Vector.Standard.from(null, 2, 3), 4), zipper.rightElements()));

    zipper = Zip.stepRight(zipper);
    assertEquals(1, zipper.leftElement());
    assertTrue(Eq.eq(List.Standard.from(null, 1), zipper.leftElements()));
    zipper = Zip.stepInside(zipper);
    assertEquals(2, zipper.depth);
    assertEquals(2, zipper.currentNode());
    zipper = Zip.stepOutside(zipper);
    assertTrue(Eq.eq(Vector.Standard.from(null, 2, 3), zipper.currentNode()));
  }

  @Test
  public void boundaryOperationsReturnSameZipperWithoutHandler() {
    Zipper root = Zip.zipper(Vector.Standard.from(null, 1));
    assertSame(root, Zip.stepLeft(root));
    assertSame(root, Zip.stepRight(root));
    assertSame(root, Zip.stepOutside(root));
    assertSame(root, Zip.deleteLeft(root));

    Zipper leaf = Zip.stepInside(root);
    assertSame(leaf, Zip.stepInside(leaf));
    assertSame(leaf, Zip.stepInsideLeft(leaf));
    Zipper empty = Zip.remove(leaf);
    assertSame(empty, Zip.remove(empty));
  }

  @Test
  public void handlerReceivesEveryBoundaryOperation() {
    RecordingHandler handler = new RecordingHandler();
    Zipper root = Zip.zipper(Vector.Standard.from(null, 1), handler);
    Zip.stepLeft(root);
    Zip.stepRight(root);
    Zip.stepOutside(root);
    Zip.deleteLeft(root);
    Zipper leaf = Zip.stepInside(root);
    Zip.stepInside(leaf);
    Zip.stepInsideLeft(leaf);
    Zip.remove(Zip.remove(leaf));
    assertEquals(
        java.util.List.of("left", "right", "outside", "delete-left", "inside", "inside-left", "delete-right"),
        handler.calls);
  }

  @Test
  public void editsPropagateToRootWithoutMutatingOriginal() {
    Vector<Object> original = Vector.Standard.from(null, 1, 2, 3);
    Zipper zipper = Zip.stepRight(Zip.stepInside(Zip.zipper(original)));
    zipper = Zip.insertLeft(zipper, 9);
    zipper = Zip.replace(zipper, 8);
    assertTrue(Eq.eq(Vector.Standard.from(null, 1, 9, 8, 3), Zip.result(zipper)));
    assertTrue(Eq.eq(Vector.Standard.from(null, 1, 2, 3), original));
    assertTrue(zipper.changed);
  }

  @Test
  public void updateElementsHandlesReplacementAndRemoval() {
    Zipper zipper = Zip.stepRight(Zip.stepInside(Zip.zipper(Vector.Standard.from(null, 1, 2, 3))));
    Zipper replaced = Zip.updateElements(zipper, List.Standard.from(null, 7, 8));
    assertTrue(Eq.eq(Vector.Standard.from(null, 1, 7, 8, 3), Zip.result(replaced)));

    Zipper removed = Zip.updateElements(zipper, List.Standard.EMPTY);
    assertTrue(Eq.eq(Vector.Standard.from(null, 1, 3), Zip.result(removed)));
    assertTrue(Eq.eq(Vector.Standard.from(null, 1, 3), Zip.result(Zip.updateElements(zipper, null))));
  }

  @Test
  public void reverseConcatAndTraversalCoverEmptyAndNestedCollections() {
    List<?> input = List.Standard.from(null, 1, 2, 3);
    assertTrue(Eq.eq(List.Standard.from(null, 3, 2, 1), Zip.reverse((List) input)));
    assertTrue(
        Eq.eq(
            List.Standard.from(null, 1, 2, 3, 4),
            Zip.concat(List.Standard.from(null, 1, 2), List.Standard.from(null, 3, 4))));

    Zipper empty = Zip.zipper(Vector.Standard.EMPTY);
    assertNull(Zip.stepNext(empty));

    Vector<Object> root = Vector.Standard.from(null, Vector.Standard.EMPTY, 1);
    Zipper found = Zip.findNext(Zip.zipper(root), z -> Integer.valueOf(1).equals(z.currentNode()));
    assertNotNull(found);
    assertEquals(1, found.currentNode());
    assertNull(Zip.findNext(found, z -> false));
  }

  @Test
  public void prewalkAndPostwalkObserveDifferentTraversalOrder() {
    Vector<Object> root = Vector.Standard.from(null, Vector.Standard.from(null, 1), 2);
    java.util.List<Object> pre = new ArrayList<>();
    java.util.List<Object> post = new ArrayList<>();
    Zip.prewalk(Zip.zipper(root), value -> { pre.add(value); return value; });
    Zip.postwalk(Zip.zipper(root), value -> { post.add(value); return value; });
    assertEquals(2, pre.size());
    assertTrue(Eq.eq(root, pre.get(0)));
    assertTrue(Eq.eq(Vector.Standard.from(null, 1), pre.get(1)));
    assertEquals(2, post.size());
    assertTrue(Eq.eq(Vector.Standard.from(null, 1), post.get(0)));
    assertTrue(Eq.eq(root, post.get(1)));
  }

  private static final class RecordingHandler extends BaseHandler {
    final java.util.List<String> calls = new ArrayList<>();
    public Zipper onStepAtLeftMost(Zipper zipper) { calls.add("left"); return zipper; }
    public Zipper onStepAtRightMost(Zipper zipper) { calls.add("right"); return zipper; }
    public Zipper onStepAtInsideMost(Zipper zipper) { calls.add("inside"); return zipper; }
    public Zipper onStepAtInsideMostLeft(Zipper zipper) { calls.add("inside-left"); return zipper; }
    public Zipper onStepAtOutsideMost(Zipper zipper) { calls.add("outside"); return zipper; }
    public Zipper onDeleteAtLeftMost(Zipper zipper) { calls.add("delete-left"); return zipper; }
    public Zipper onDeleteAtRightMost(Zipper zipper) { calls.add("delete-right"); return zipper; }
  }
}
