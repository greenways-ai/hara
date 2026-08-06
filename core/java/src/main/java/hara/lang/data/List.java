package hara.lang.data;

import hara.lang.base.Iter;
import hara.lang.base.primitive.Array;
import hara.lang.data.types.IVectorType;
import hara.lang.data.types.ObjMutable;
import hara.lang.data.types.ObjPersistent;
import hara.lang.protocol.*;

import java.util.Arrays;
import java.util.Iterator;
import java.util.NoSuchElementException;

import static hara.lang.base.primitive.Bits.log2Ceil;

public interface List<E> extends IVectorType<E> {

  public static int CHUNK_SIZE = 32;
  public static int DEFAULT_CAPACITY = 4;

  public interface Base<E> extends List<E>, IAssoc<Integer, E>, IObjType {

    @Override
    default String startString() {
      return "(";
    }

    @Override
    default String endString() {
      return ")";
    }
  }

  class Chunk<E> {
    final Object[] array;
    final int offset;
    final int count;
    final Chunk<E> next;

    public Chunk(Object[] array, int offset, int count, Chunk<E> next) {
      this.array = array;
      this.offset = offset;
      this.count = count;
      this.next = next;
    }
  }

  class Standard<E> extends ObjPersistent implements Base<E>, IToMutable {

    final Chunk<E> _head;
    final int _size;

    public static final Standard EMPTY = new Standard(null, null, 0);

    public Standard(IMetadata meta, Chunk<E> head, int size) {
      super(meta);
      _head = head;
      _size = size;
    }

    @SuppressWarnings("unchecked")
    public static <E> Standard<E> from(IMetadata meta, E... elements) {
      Mutable<E> m = new Mutable<>(meta);
      for (E e : elements) {
        m.pushLast(e);
      }
      return m.toPersistent();
    }

    @Override
    public Standard<E> conj(E v) {
      return pushLast(v);
    }

    @Override
    public Standard<E> cons(E v) {
      return pushFirst(v);
    }

    public static <E> Standard<E> into(Iterator<E> it) {
      return Mutable.into(it).toPersistent();
    }

    public static <E> Standard<E> into(Standard<E> coll, Iterator<E> it) {
      return Mutable.into(coll.toMutable(), it).toPersistent();
    }

    @Override
    public Standard<E> withMeta(IMetadata meta) {
      return new Standard<>(meta, _head, _size);
    }

    @Override
    public Standard<E> pushFirst(E e) {
      if (_head != null && _head.offset > 0) {
        Object[] newArr = _head.array.clone();
        newArr[_head.offset - 1] = e;
        Chunk<E> newHead = new Chunk<>(newArr, _head.offset - 1, _head.count + 1, _head.next);
        return new Standard<>(_meta, newHead, _size + 1);
      } else {
        Object[] newArr = new Object[CHUNK_SIZE];
        newArr[CHUNK_SIZE - 1] = e;
        Chunk<E> newHead = new Chunk<>(newArr, CHUNK_SIZE - 1, 1, _head);
        return new Standard<>(_meta, newHead, _size + 1);
      }
    }

    @Override
    public Standard<E> popFirst() {
      if (_size == 0) return this;

      if (_head.count > 1) {
        Chunk<E> newHead = new Chunk<>(_head.array, _head.offset + 1, _head.count - 1, _head.next);
        return new Standard<>(_meta, newHead, _size - 1);
      } else {
        return new Standard<>(_meta, _head.next, _size - 1);
      }
    }

    @Override
    public Standard<E> pushLast(E e) {
      return (Standard<E>) toMutable().pushLast(e).toPersistent();
    }

    @Override
    public Standard<E> popLast() {
      return (Standard<E>) toMutable().popLast().toPersistent();
    }

    @Override
    public Standard<E> assoc(Integer idx, E v) {
      return (Standard<E>) toMutable().assoc(idx, v).toPersistent();
    }

    @Override
    public Standard<E> empty() {
      return EMPTY;
    }

    @Override
    public Mutable<E> toMutable() {
      return new Mutable<>(_meta, _head, _size);
    }

    @Override
    public long count() {
      return _size;
    }

    @SuppressWarnings("unchecked")
    @Override
    public E nth(long i) {
      if (i < 0 || i >= _size) {
        throw new IndexOutOfBoundsException();
      }
      Chunk<E> curr = _head;
      long idx = i;
      while (curr != null) {
        if (idx < curr.count) {
          return (E) curr.array[curr.offset + (int) idx];
        }
        idx -= curr.count;
        curr = curr.next;
      }
      throw new IndexOutOfBoundsException();
    }

    @Override
    public Iterator<E> iterator() {
      return new Iterator<E>() {
        Chunk<E> current = _head;
        int idx = 0;

        @Override
        public boolean hasNext() {
          return current != null && (current.next != null || idx < current.count);
        }

        @Override
        public E next() {
          if (current == null) throw new NoSuchElementException();
          if (idx >= current.count) {
            current = current.next;
            idx = 0;
            if (current == null) throw new NoSuchElementException();
          }
          return (E) current.array[current.offset + idx++];
        }
      };
    }
  }

  class Mutable<E> extends ObjMutable implements Base<E>, IToPersistent {

    // Use Ring Buffer logic for efficient Mutable operations
    private Object[] _elements;
    private int _mask;
    private int _size, _offset;

    public Mutable() {
      this(null, DEFAULT_CAPACITY);
    }

    public Mutable(IMetadata meta) {
      this(meta, DEFAULT_CAPACITY);
    }

    public Mutable(IMetadata meta, int capacity) {
      super(meta);
      _elements = new Object[Math.max(1, 1 << log2Ceil(capacity))];
      _size = 0;
      _offset = 0;
      _mask = _elements.length - 1;
    }

    // Constructor for converting from Persistent Chunk list
    public Mutable(IMetadata meta, Chunk<E> head, int size) {
      super(meta);
      _size = size;
      _elements = new Object[Math.max(1, 1 << log2Ceil(size + 1))];
      _mask = _elements.length - 1;
      _offset = 0;

      // Flatten chunks into ring buffer
      int i = 0;
      Chunk<E> curr = head;
      while (curr != null) {
        System.arraycopy(curr.array, curr.offset, _elements, i, curr.count);
        i += curr.count;
        curr = curr.next;
      }
    }

    @SafeVarargs
    public static <E> Mutable<E> from(IMetadata meta, E... objs) {
      Mutable<E> mut = new Mutable<E>(meta, objs.length);
      return Array.reduce((arr, e) -> (Mutable<E>) arr.pushLast(e), mut, objs);
    }

    public static <E> Mutable<E> into(Iterator<E> it) {
      return into(new Mutable<E>(), it);
    }

    public static <E> Mutable<E> into(Mutable<E> coll, Iterator<E> it) {
      return Iter.reduce(it, coll, (m, e) -> (Mutable<E>) m.conj(e));
    }

    // Helper for resizing
    private void resize(int newCapacity) {
      Object[] newArr = new Object[newCapacity];
      int tailLen = Math.min(_size, _elements.length - _offset);
      System.arraycopy(_elements, _offset, newArr, 0, tailLen);
      if (_size > tailLen) {
        System.arraycopy(_elements, 0, newArr, tailLen, _size - tailLen);
      }
      _elements = newArr;
      _offset = 0;
      _mask = newCapacity - 1;
    }

    @Override
    public Mutable<E> pushFirst(E e) {
      if (_size == _elements.length) {
        resize(_size << 1);
      }
      _offset = (_offset - 1) & _mask;
      _elements[_offset] = e;
      _size++;
      return this;
    }

    @Override
    public Mutable<E> pushLast(E e) {
      if (_size == _elements.length) {
        resize(_size << 1);
      }
      _elements[(_offset + _size) & _mask] = e;
      _size++;
      return this;
    }

    @Override
    public Mutable<E> popFirst() {
      if (_size == 0) return this;
      _elements[_offset] = null;
      _offset = (_offset + 1) & _mask;
      _size--;
      return this;
    }

    @Override
    public Mutable<E> popLast() {
      if (_size == 0) return this;
      _elements[(_offset + _size - 1) & _mask] = null;
      _size--;
      return this;
    }

    @SuppressWarnings("unchecked")
    @Override
    public E nth(long i) {
      if (i < 0 || i >= _size) throw new IndexOutOfBoundsException();
      return (E) _elements[(_offset + (int) i) & _mask];
    }

    @Override
    public Mutable<E> assoc(Integer idx, E v) {
      if (idx == _size) return pushLast(v);
      if (idx < 0 || idx > _size) throw new IndexOutOfBoundsException();
      _elements[(_offset + idx) & _mask] = v;
      return this;
    }

    @Override
    public Mutable<E> empty() {
      _size = 0;
      _offset = 0;
      Arrays.fill(_elements, null);
      return this;
    }

    @Override
    public Mutable<E> conj(E v) {
      return pushLast(v);
    }

    @Override
    public Mutable<E> cons(E v) {
      return pushFirst(v);
    }

    public Mutable<E> conjAll(Iterator<E> it) {
      return Iter.reduce(it, this, (m, e) -> (Mutable<E>) m.conj(e));
    }

    public Standard<E> toPersistentRaw() {
      return toPersistent();
    }

    @Override
    public Standard<E> toPersistent() {
      // Convert Ring Buffer to Chunked List
      Chunk<E> currentNext = null;

      int remaining = _size;
      while (remaining > 0) {
        int countInChunk = remaining % CHUNK_SIZE;
        if (countInChunk == 0) countInChunk = CHUNK_SIZE;

        Object[] arr = new Object[CHUNK_SIZE];

        // Fill array
        int startLogical = remaining - countInChunk;
        for (int i = 0; i < countInChunk; i++) {
          arr[i] = _elements[(_offset + startLogical + i) & _mask];
        }

        // Create chunk
        // offset is 0 because we filled from start of array
        currentNext = new Chunk<>(arr, 0, countInChunk, currentNext);

        remaining -= countInChunk;
      }

      return new Standard<>(_meta, currentNext, _size);
    }

    @Override
    public long count() {
      return _size;
    }

    @Override
    public Iterator<E> iterator() {
      return new Iterator<E>() {
        int idx = 0;

        @Override
        public boolean hasNext() {
          return idx < _size;
        }

        @SuppressWarnings("unchecked")
        @Override
        public E next() {
          if (idx >= _size) throw new NoSuchElementException();
          return (E) _elements[(_offset + idx++) & _mask];
        }
      };
    }
  }
}
