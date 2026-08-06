package hara.lang.data;

import org.junit.Test;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertNull;

public class QueueTest {

  @Test
  public void testStandardQueue() {
    Queue.Standard<Integer> queue = Queue.Standard.from(null, 1, 2, 3);
    assertEquals(3, queue.count());
    assertEquals(Integer.valueOf(1), queue.peekFirst());
    assertEquals(Integer.valueOf(3), queue.peekLast());

    queue = queue.pushLast(4);
    assertEquals(4, queue.count());
    assertEquals(Integer.valueOf(4), queue.peekLast());

    queue = queue.popFirst();
    assertEquals(3, queue.count());
    assertEquals(Integer.valueOf(2), queue.peekFirst());

    queue = queue.popLast();
    assertEquals(2, queue.count());
    assertEquals(Integer.valueOf(3), queue.peekLast());
  }

  @Test
  public void testMutableQueue() {
    Queue.Mutable<Integer> queue = Queue.Mutable.from(null, 1, 2, 3);
    assertEquals(3, queue.count());
    assertEquals(Integer.valueOf(1), queue.peekFirst());
    assertEquals(Integer.valueOf(3), queue.peekLast());

    queue.pushLast(4);
    assertEquals(4, queue.count());
    assertEquals(Integer.valueOf(4), queue.peekLast());

    queue.popFirst();
    assertEquals(3, queue.count());
    assertEquals(Integer.valueOf(2), queue.peekFirst());

    queue.popLast();
    assertEquals(2, queue.count());
    assertEquals(Integer.valueOf(3), queue.peekLast());
  }

  @Test
  public void testEmptyQueue() {
    Queue.Standard<Integer> queue = Queue.Standard.empty(null);
    assertEquals(0, queue.count());
    assertNull(queue.peekFirst());
    assertNull(queue.peekLast());

    Queue.Mutable<Integer> mutableQueue = Queue.Mutable.empty(null);
    assertEquals(0, mutableQueue.count());
    assertNull(mutableQueue.peekFirst());
    assertNull(mutableQueue.peekLast());
  }

  @Test
  public void testQueueIteration() {
    Queue.Standard<Integer> queue = Queue.Standard.from(null, 1, 2, 3, 4, 5);

    int expected = 1;
    for (Integer actual : queue) {
      assertEquals(Integer.valueOf(expected), actual);
      expected++;
    }
  }

  @Test
  public void testEmptyQueuePop() {
    Queue.Standard<Integer> queue = Queue.Standard.empty(null);
    assertEquals(0, queue.popFirst().count());
    assertEquals(0, queue.popLast().count());

    Queue.Mutable<Integer> mutableQueue = Queue.Mutable.empty(null);
    mutableQueue.popFirst();
    mutableQueue.popLast();
    assertEquals(0, mutableQueue.count());
  }
}
