package hara.lang.data;

import hara.lang.base.Ex;
import hara.lang.base.Iter;
import hara.lang.base.primitive.Array;
import hara.lang.data.types.ILinearView;
import hara.lang.data.types.IVectorType;
import hara.lang.data.types.ObjMutable;
import hara.lang.data.types.ObjPersistent;
import hara.lang.protocol.*;

import java.util.Iterator;
import java.util.NoSuchElementException;
import java.util.concurrent.atomic.AtomicReference;

public interface Vector<E> extends IVectorType<E>, IAssoc<Integer, E> {

  public interface S {

    // STATIC METHODS
    public static Node editableRoot(Node node) {
      return new Node(new AtomicReference<>(Thread.currentThread()), node.array.clone());
    }

    public static Node editableNode(Node node, AtomicReference<Thread> edit) {
      return (node.edit == edit) ? node : new Node(edit, node.array.clone());
    }

    public static int tailoff(int size) {
      return (size < Node.NODE_MAXLEN) ? 0 : ((size - 1) >>> Node.NODE_SHIFT) << Node.NODE_SHIFT;
    }

    @SuppressWarnings("unchecked")
    public static <E> E[] getNodeArrayFor(
        Node root, int size, int shift, E[] tail, int i, boolean editable) {
      if (i >= 0 && i < size) {
        if (i >= S.tailoff(size)) {
          return tail;
        }
        Node node = root;
        for (int level = shift; level > 0; level -= Node.NODE_SHIFT) {
          node =
              (editable == false)
                  ? (Node) node.array[(i >>> level) & Node.NODE_MASK]
                  : editableNode((Node) node.array[(i >>> level) & Node.NODE_MASK], root.edit);
        }
        return (E[]) node.array;
      }
      throw new IndexOutOfBoundsException();
    }

    public static Node newPath(AtomicReference<Thread> edit, int level, Node node) {
      if (level == 0) return node;
      var ret = new Node(edit);
      ret.array[0] = newPath(edit, level - Node.NODE_SHIFT, node);
      return ret;
    }

    public static Object[] makeTail(Object[] tail) {
      var ret = new Object[32];
      System.arraycopy(tail, 0, ret, 0, tail.length);
      return ret;
    }

    public static Object[] newTail(Object[] tail, int i, Object val) {
      var newTail = new Object[tail.length];
      System.arraycopy(tail, 0, newTail, 0, tail.length);
      newTail[i & Node.NODE_MASK] = val;
      return newTail;
    }

    public static Object[] newTailAppend(Object[] tail, Object val) {
      var newTail = new Object[tail.length + 1];
      System.arraycopy(tail, 0, newTail, 0, tail.length);
      newTail[tail.length] = val;
      return newTail;
    }

    public static Object[] trimTail(Object[] tail, int size) {
      var trimmed = new Object[size - tailoff(size)];
      System.arraycopy(tail, 0, trimmed, 0, trimmed.length);
      return trimmed;
    }

    public static Node pushTail(
        Node root, int size, int level, Node parent, Node _tailnode, boolean editable) {
      if (editable == true) parent = S.editableNode(parent, root.edit);
      int subidx = ((size - 1) >>> level) & Node.NODE_MASK;
      var ret = new Node(parent.edit, parent.array.clone());
      Node nodeToInsert;
      if (level == Node.NODE_SHIFT) {
        nodeToInsert = _tailnode;
      } else {
        Node child = (Node) parent.array[subidx];
        nodeToInsert =
            (child == null)
                ? S.newPath(root.edit, level - Node.NODE_SHIFT, _tailnode)
                : pushTail(root, size, level - Node.NODE_SHIFT, child, _tailnode, editable);
      }
      ret.array[subidx] = nodeToInsert;
      return ret;
    }

    public static Node popTail(Node root, int size, int level, Node node, boolean editable) {
      if (editable == true) node = S.editableNode(node, root.edit);
      int subidx = ((size - 2) >>> level) & Node.NODE_MASK;
      if (level > Node.NODE_SHIFT) {
        Node newchild =
            popTail(root, size, level - Node.NODE_SHIFT, (Node) node.array[subidx], editable);
        if (newchild == null && subidx == 0) return null;
        else {
          node.array[subidx] = newchild;
          return node;
        }
      } else if (subidx == 0) return null;
      else {
        node.array[subidx] = null;
        return node;
      }
    }

    public static Node assoc(Node node, int level, int i, Object val) {
      var ret = new Node(node.edit, node.array.clone());
      if (level == 0) {
        ret.array[i & Node.NODE_MASK] = val;
      } else {
        int subidx = (i >>> level) & Node.NODE_MASK;
        ret.array[subidx] = assoc((Node) node.array[subidx], level - Node.NODE_SHIFT, i, val);
      }
      return ret;
    }
  }

  public class Node {
    public static final int NODE_SHIFT = 5;
    public static final int NODE_MAXLEN = 1 << NODE_SHIFT;
    public static final int NODE_MASK = NODE_MAXLEN - 1;

    public static final AtomicReference<Thread> NOEDIT = new AtomicReference<>(null);
    public static final Node EMPTY = new Node(NOEDIT, new Object[NODE_MAXLEN]);

    // DATA
    public final transient AtomicReference<Thread> edit;
    public final Object[] array;

    // CONSTRUCTORS
    public Node(AtomicReference<Thread> edit, Object[] array) {
      this.edit = edit;
      this.array = array;
    }

    public Node(AtomicReference<Thread> edit) {
      this.edit = edit;
      this.array = new Object[NODE_MAXLEN];
    }
  }

  // CONSTANTS
  public interface Base<E> extends Vector<E>, IObjType {

    public Node _root();

    public int _size();

    public int _shift();

    public E[] _tail();

    @Override
    default Base<E> conj(E e) {
      return (Base<E>) pushLast(e);
    }

    @Override
    default long count() {
      return _size();
    }

    default Iterator<E> rangedIterator(final int start, final int end) {
      return new Iterator<E>() {
        int i = start;
        int base = i - (i % 32);
        E[] array =
            (start < count())
                ? S.getNodeArrayFor(_root(), _size(), _shift(), _tail(), i, false)
                : null;

        @Override
        public boolean hasNext() {
          return i < end;
        }

        @Override
        public E next() {
          if (i < end) {
            if (i - base == 32) {
              array = S.getNodeArrayFor(_root(), _size(), _shift(), _tail(), i, false);
              base += 32;
            }
            return array[i++ & 0x01f];
          } else {
            throw new NoSuchElementException();
          }
        }

        @Override
        public void remove() {
          throw new UnsupportedOperationException();
        }
      };
    }

    @Override
    default Iterator<E> iterator() {
      return rangedIterator(0, _size());
    }

    @Override
    default IPushFirst<E> pushFirst(E e) {
      throw new Ex.Unsupported();
    }

    @Override
    default IPopFirst popFirst() {
      throw new Ex.Unsupported();
    }
  }

  public class Mutable<E> extends ObjMutable implements Base<E>, IToPersistent {

    private int _size;
    private int _shift;
    private transient Node _root;
    private E[] _tail;

    @SuppressWarnings("unchecked")
    public Mutable(Base<E> v) {
      this(v.meta(), v._size(), v._shift(), S.editableRoot(v._root()), (E[]) S.makeTail(v._tail()));
    }

    public Mutable(IMetadata meta, int size, int shift, Node root, E[] tail) {
      super(meta);
      _size = size;
      _shift = shift;
      _root = root;
      _tail = tail;
    }

    @SuppressWarnings("unchecked")
    public static <E> Mutable<E> from(IMetadata meta, E... objs) {
      var vec = empty(meta);
      return Array.reduce((v, e) -> v.pushLast(e), vec, objs);
    }

    @SuppressWarnings("unchecked")
    public static <E> Mutable<E> into(Iterator<E> it) {
      return into(empty(null), it);
    }

    public static <E> Mutable<E> into(Mutable<E> coll, Iterator<E> it) {
      return Iter.reduce(it, coll, (m, e) -> m.pushLast(e));
    }

    @SuppressWarnings({"unchecked", "rawtypes"})
    public static Mutable empty(IMetadata meta) {
      return new Mutable(
          meta, 0, Node.NODE_SHIFT, S.editableRoot(Node.EMPTY), S.makeTail(new Object[] {}));
    }

    private void checkEditable() {
      if (_root.edit.get() == null) {
        throw new IllegalStateException("Mutable used after immutable! call");
      }
    }

    @Override
    public int _size() {
      return _size;
    }

    @Override
    public int _shift() {
      return _shift;
    }

    @Override
    public Node _root() {
      checkEditable();
      return _root;
    }

    @Override
    public E[] _tail() {
      checkEditable();
      return _tail;
    }

    @Override
    @SuppressWarnings("unchecked")
    public Mutable<E> pushLast(E val) {
      checkEditable();
      int i = _size;
      // room in _tail?
      if (i - S.tailoff(_size) < Node.NODE_MAXLEN) {
        _tail[i & Node.NODE_MASK] = val;
        ++_size;
        return this;
      }
      // full _tail, push into tree
      Node new_root;
      Node _tailnode = new Node(_root.edit, _tail);
      _tail = (E[]) new Object[Node.NODE_MAXLEN];
      _tail[0] = val;
      int new_shift = _shift;
      // overflow _root?
      if ((_size >>> Node.NODE_SHIFT) > (1 << _shift)) {
        new_root = new Node(_root.edit);
        new_root.array[0] = _root;
        new_root.array[1] = S.newPath(_root.edit, _shift, _tailnode);
        new_shift += Node.NODE_SHIFT;
      } else new_root = S.pushTail(_root, _size, _shift, _root, _tailnode, true);
      _root = new_root;
      _shift = new_shift;
      ++_size;
      return this;
    }

    @Override
    public E nth(long i) {
      checkEditable();
      E[] node = S.getNodeArrayFor(_root, _size, _shift, _tail, (int) i, true);
      return node[(int) i & Node.NODE_MASK];
    }

    @Override
    public Mutable<E> assoc(Integer idx, E e) {
      checkEditable();
      if (idx >= 0 && idx < _size) {
        E[] node = S.getNodeArrayFor(_root, _size, _shift, _tail, idx, true);
        node[idx & Node.NODE_MASK] = e;
        return this;
      }
      if (idx == _size) {
        return pushLast(e);
      }
      throw new IndexOutOfBoundsException();
    }

    @Override
    public Mutable<E> popLast() {
      checkEditable();
      if (_size == 0) throw new IllegalStateException("Can't pop empty vector");
      if (_size == 1) {
        _size = 0;
        return this;
      }
      int i = _size - 1;
      if ((i & Node.NODE_MASK) > 0) {
        _size--;
        return this;
      }

      E[] newtail = S.getNodeArrayFor(_root, _size, _shift, _tail, _size - 2, true);

      Node newroot = S.popTail(_root, _size, _shift, _root, true);
      int newshift = _shift;
      if (newroot == null) {
        newroot = new Node(_root.edit);
      }
      if (_shift > Node.NODE_SHIFT && newroot.array[1] == null) {
        newroot = S.editableNode((Node) newroot.array[0], _root.edit);
        newshift -= Node.NODE_SHIFT;
      }
      _root = newroot;
      _shift = newshift;
      _size--;
      _tail = newtail;
      return this;
    }

    @SuppressWarnings("unchecked")
    @Override
    public Standard<E> toPersistent() {
      _root.edit.set(null);
      return new Standard<>(_meta, _size, _shift, _root, (E[]) S.trimTail(_tail, _size));
    }

    @SuppressWarnings("unchecked")
    @Override
    public Mutable<E> empty() {
      _size = 0;
      _shift = Node.NODE_SHIFT;
      _root = Node.EMPTY;
      _tail = (E[]) new Object[] {};
      return this;
    }
  }

  public class Standard<E> extends ObjPersistent implements Base<E>, IToMutable, ILinearView<E> {

    // STATIC
    public static final Standard<Object> EMPTY =
        new Standard<>(null, 0, Node.NODE_SHIFT, Node.EMPTY, new Object[] {});

    // INSTANCE
    private final int _size;
    private final int _shift;
    private final transient Node _root;
    private final E[] _tail;

    public Standard(Base<E> v) {
      this(v.meta(), v._size(), v._shift(), v._root(), v._tail());
    }

    protected Standard(IMetadata meta, int size, int shift, Node root, E[] tail) {
      super(meta);
      _size = size;
      _shift = shift;
      _root = root;
      _tail = tail;
    }

    @SuppressWarnings("unchecked")
    public static <E> Standard<E> empty(IMetadata meta) {
      Standard<E> ret = (Standard<E>) EMPTY;
      return (meta == null) ? ret : ret.withMeta(meta);
    }

    @SuppressWarnings("unchecked")
    public static <E> Standard<E> from(IMetadata meta, E... objs) {
      return Mutable.from(meta, objs).toPersistent();
    }

    public static <E> Standard<E> into(Iterator<E> it) {
      return Mutable.into(it).toPersistent();
    }

    public static <E> Standard<E> into(Standard<E> coll, Iterator<E> it) {
      return Mutable.into(coll.toMutable(), it).toPersistent();
    }

    @Override
    public int _size() {
      return _size;
    }

    @Override
    public int _shift() {
      return _shift;
    }

    @Override
    public Node _root() {
      return _root;
    }

    @Override
    public E[] _tail() {
      return _tail;
    }

    @Override
    public Standard<E> withMeta(IMetadata meta) {
      return (meta() == meta) ? this : new Standard<E>(meta, _size, _shift, _root, _tail);
    }

    @Override
    public E nth(long i) {
      E[] node = S.getNodeArrayFor(_root, _size, _shift, _tail, (int) i, false);
      return node[(int) i & Node.NODE_MASK];
    }

    @SuppressWarnings("unchecked")
    @Override
    public Standard<E> assoc(Integer i, E val) {
      if (i >= 0 && i < _size) {
        return (i >= S.tailoff(_size))
            ? new Standard<>(_meta, _size, _shift, _root, (E[]) S.newTail(_tail, i, val))
            : new Standard<>(_meta, _size, _shift, S.assoc(_root, _shift, i, val), _tail);
      }
      if (i == _size) {
        return pushLast(val);
      }
      throw new IndexOutOfBoundsException();
    }

    @SuppressWarnings("unchecked")
    @Override
    public Standard<E> pushLast(E val) {
      if (_size - S.tailoff(_size) < Node.NODE_MAXLEN) {
        return new Standard<>(_meta, _size + 1, _shift, _root, (E[]) S.newTailAppend(_tail, val));
      }

      Node new_root;
      Node _tailnode = new Node(_root.edit, _tail);
      int new_shift = _shift;
      if ((_size >>> Node.NODE_SHIFT) > (1 << _shift)) {
        new_root = new Node(_root.edit);
        new_root.array[0] = _root;
        new_root.array[1] = S.newPath(_root.edit, _shift, _tailnode);
        new_shift += Node.NODE_SHIFT;
      } else {
        new_root = S.pushTail(_root, _size, _shift, _root, _tailnode, false);
      }
      return new Standard<>(_meta, _size + 1, new_shift, new_root, (E[]) new Object[] {val});
    }

    @SuppressWarnings("unchecked")
    @Override
    public Standard<E> popLast() {
      if (_size == 0) throw new IllegalStateException("Can't pop empty vector");
      if (_size == 1) return empty(_meta);
      if (_size - S.tailoff(_size) > 1) {
        E[] newTail = (E[]) new Object[_tail.length - 1];
        System.arraycopy(_tail, 0, newTail, 0, newTail.length);
        return new Standard<E>(_meta, _size - 1, _shift, _root, newTail);
      }
      E[] newtail = S.getNodeArrayFor(_root, _size, _shift, _tail, _size - 2, false);

      Node new_root = S.popTail(_root, _size, _shift, _root, false);
      int new_shift = _shift;
      if (new_root == null) {
        new_root = Node.EMPTY;
      }
      if (_shift > Node.NODE_SHIFT && new_root.array[1] == null) {
        new_root = (Node) new_root.array[0];
        new_shift -= Node.NODE_SHIFT;
      }
      return new Standard<E>(_meta, _size - 1, new_shift, new_root, newtail);
    }

    @Override
    public Standard<E> empty() {
      return empty(_meta);
    }

    @Override
    public Mutable<E> toMutable() {
      return new Mutable<E>(this);
    }

    @Override
    public SubView<E> subview(int start, int end) {
      if ((end > _size - 1) || (start < 0)) throw new IndexOutOfBoundsException();
      return new SubView<E>(_meta, this, start, end);
    }
  }

  public class SubView<E> extends ObjPersistent
      implements Vector<E>, IAssoc<Integer, E>, IObjType, ILinearView<E> {

    final Vector<E> _v;
    final int _start;
    final int _end;

    public SubView(IMetadata meta, Vector<E> v, int start, int end) {
      super(meta);
      _v = v;
      _start = start;
      _end = end;
    }

    @Override
    public Vector<E> empty() {
      return Standard.empty(_meta);
    }

    @Override
    public SubView<E> withMeta(IMetadata meta) {
      return (meta() == meta) ? this : new SubView<E>(meta, _v, _start, _end);
    }

    @Override
    public E nth(long i) {
      if ((_start + i >= _end) || (i < 0)) throw new IndexOutOfBoundsException();
      return _v.nth(_start + i);
    }

    @Override
    public SubView<E> pushLast(E e) {
      return new SubView<E>(_meta, (Vector<E>) _v.assoc(_end, e), _start, _end + 1);
    }

    @Override
    public IPopLast popLast() {
      if (_end - _start == 0) {
        return this;
      } else {
        return new SubView<E>(_meta, _v, _start, _end - 1);
      }
    }

    @Override
    public SubView<E> assoc(Integer idx, E e) {
      if (_start + idx > _end) throw new IndexOutOfBoundsException();
      else if (_start + idx == _end) return pushLast(e);
      return new SubView<E>(_meta, (Vector<E>) _v.assoc(_start + idx, e), _start, _end);
    }

    @Override
    public Iterator<E> iterator() {
      return ((Standard<E>) _v).rangedIterator(_start, _end);
    }

    @Override
    public long count() {
      return _end - _start;
    }

    @Override
    public Vector<E> subview(int start, int end) {
      if ((end > (_end - _start) - 1 || (start < 0))) {
        throw new IndexOutOfBoundsException();
      }
      return new SubView<E>(_meta, _v, (_start + start), (_start + end));
    }
  }
}
