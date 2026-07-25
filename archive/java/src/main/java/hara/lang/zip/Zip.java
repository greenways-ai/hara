package hara.lang.zip;

import hara.lang.data.List;

public class Zip {

  public static Zipper zipper(Object root) {
    return zipper(root, new ZipContext(), null);
  }

  public static Zipper zipper(Object root, IZipHandler handler) {
    return zipper(root, new ZipContext(), handler);
  }

  public static Zipper zipper(Object root, IZipContext context, IZipHandler handler) {
    return new Zipper(
        null,
        List.Standard.EMPTY,
        (List) List.Standard.EMPTY.conj(context.wrapData(root)),
        0,
        false,
        context,
        handler);
  }

  public static Zipper stepLeft(Zipper zipper) {
    if (zipper.atLeftMost()) {
      if (zipper.handler != null) {
        return zipper.handler.onStepAtLeftMost(zipper);
      }
      return zipper;
    } else {
      Object elem = zipper.left.peekFirst();
      List newLeft = (List) zipper.left.popFirst();
      List newRight = (List) zipper.right.cons(elem);
      return new Zipper(
          zipper.parent,
          newLeft,
          newRight,
          zipper.depth,
          zipper.changed,
          zipper.context,
          zipper.handler);
    }
  }

  public static Zipper stepRight(Zipper zipper) {
    if (zipper.atRightMost()) {
      if (zipper.handler != null) {
        return zipper.handler.onStepAtRightMost(zipper);
      }
      return zipper;
    } else {
      Object elem = zipper.right.peekFirst();
      List newLeft = (List) zipper.left.cons(elem);
      List newRight = (List) zipper.right.popFirst();
      return new Zipper(
          zipper.parent,
          newLeft,
          newRight,
          zipper.depth,
          zipper.changed,
          zipper.context,
          zipper.handler);
    }
  }

  public static Zipper stepInside(Zipper zipper) {
    if (zipper.atInsideMost()) {
      if (zipper.handler != null) {
        return zipper.handler.onStepAtInsideMost(zipper);
      }
      return zipper;
    } else {
      Object elem = zipper.right.peekFirst();
      List children = zipper.context.listElements(elem);
      return new Zipper(
          zipper,
          List.Standard.EMPTY,
          children,
          zipper.depth + 1,
          zipper.changed,
          zipper.context,
          zipper.handler);
    }
  }

  public static Zipper stepInsideLeft(Zipper zipper) {
    if (zipper.atInsideMostLeft()) {
      if (zipper.handler != null) {
        return zipper.handler.onStepAtInsideMostLeft(zipper);
      }
      return zipper;
    } else {
      Object elem = zipper.left.peekFirst();
      List children = zipper.context.listElements(elem);
      Zipper parent =
          new Zipper(
              zipper.parent,
              (List) zipper.left.popFirst(),
              (List) zipper.right.cons(elem),
              zipper.depth,
              zipper.changed,
              zipper.context,
              zipper.handler);
      return new Zipper(
          parent,
          reverse(children),
          List.Standard.EMPTY,
          zipper.depth + 1,
          zipper.changed,
          zipper.context,
          zipper.handler);
    }
  }

  public static Zipper stepOutside(Zipper zipper) {
    if (zipper.atOutsideMost()) {
      if (zipper.handler != null) {
        return zipper.handler.onStepAtOutsideMost(zipper);
      }
      return zipper;
    } else {
      List children = concat(reverse(zipper.left), zipper.right);
      Zipper parent = zipper.parent;
      if (zipper.changed) {
        Object oldNode = parent.right.peekFirst();
        Object newNode = zipper.context.updateElements(oldNode, children);
        return new Zipper(
            parent.parent,
            parent.left,
            (List) ((List) parent.right.popFirst()).cons(newNode),
            parent.depth,
            true,
            parent.context,
            parent.handler);
      } else {
        return parent;
      }
    }
  }

  public static Zipper insertLeft(Zipper zipper, Object data) {
    Object elem = zipper.context.createElement(data);
    List newLeft = (List) zipper.left.cons(elem);
    return new Zipper(
        zipper.parent, newLeft, zipper.right, zipper.depth, true, zipper.context, zipper.handler);
  }

  public static Zipper insertAndFocus(Zipper zipper, Object data) {
    Object elem = zipper.context.createElement(data);
    List newRight = (List) zipper.right.cons(elem);
    return new Zipper(
        zipper.parent, zipper.left, newRight, zipper.depth, true, zipper.context, zipper.handler);
  }

  public static Zipper deleteLeft(Zipper zipper) {
    if (zipper.atLeftMost()) {
      if (zipper.handler != null) {
        return zipper.handler.onDeleteAtLeftMost(zipper);
      }
      return zipper;
    } else {
      List newLeft = (List) zipper.left.popFirst();
      return new Zipper(
          zipper.parent, newLeft, zipper.right, zipper.depth, true, zipper.context, zipper.handler);
    }
  }

  public static Zipper remove(Zipper zipper) {
    if (zipper.right.count() == 0) {
      if (zipper.handler != null) {
        return zipper.handler.onDeleteAtRightMost(zipper);
      }
      return zipper;
    } else {
      List newRight = (List) zipper.right.popFirst();
      return new Zipper(
          zipper.parent, zipper.left, newRight, zipper.depth, true, zipper.context, zipper.handler);
    }
  }

  public static Zipper replace(Zipper zipper, Object data) {
    if (zipper.right.count() == 0) {
      return zipper;
    }
    return insertAndFocus(remove(zipper), data);
  }

  public static List reverse(List list) {
    List.Mutable rev = new List.Mutable();
    for (Object o : list) {
      rev = (List.Mutable) rev.cons(o);
    }
    return rev.toPersistent();
  }

  public static List concat(List a, List b) {
    List.Mutable result = new List.Mutable();
    for (Object o : a) {
      result = (List.Mutable) result.conj(o);
    }
    for (Object o : b) {
      result = (List.Mutable) result.conj(o);
    }
    return result.toPersistent();
  }

  public static Zipper stepNext(Zipper zipper) {
    if (zipper == null) {
      return null;
    }

    // 1. Try to go down into a non-empty container
    if (!zipper.atInsideMost() && !zipper.isEmptyContainer()) {
      return stepInside(zipper);
    }

    // 2. Try to go right
    if (!zipper.atRightMost()) {
      return stepRight(zipper);
    }

    // 3. Go up and right until we succeed or hit the top
    Zipper p = zipper;
    while (!p.atOutsideMost()) {
      p = stepOutside(p);
      if (!p.atRightMost()) {
        return stepRight(p);
      }
    }

    return null; // End of traversal
  }

  public static Object result(Zipper zipper) {
    Zipper current = zipper;
    while (current.parent != null) {
      current = stepOutside(current);
    }
    return current.currentNode();
  }

  public static Zipper updateElements(Zipper zipper, hara.lang.data.List newElements) {
    if (newElements == null || newElements.count() == 0) {
      return remove(zipper);
    }

    hara.lang.data.List originalRightTail = (hara.lang.data.List) zipper.right.popFirst();
    if (originalRightTail == null) {
      originalRightTail = hara.lang.data.List.Standard.EMPTY;
    }

    hara.lang.data.List newRight = concat(newElements, originalRightTail);

    return new Zipper(
        zipper.parent, zipper.left, newRight, zipper.depth, true, zipper.context, zipper.handler);
  }

  public static Zipper find(
      Zipper zipper,
      java.util.function.Function<Zipper, Zipper> move,
      java.util.function.Predicate<Zipper> pred) {
    Zipper current = move.apply(zipper);
    while (current != null) {
      if (pred.test(current)) {
        return current;
      }
      current = move.apply(current);
    }
    return null;
  }

  public static Zipper findNext(Zipper zipper, java.util.function.Predicate<Zipper> pred) {
    return find(zipper, Zip::stepNext, pred);
  }

  public static Zipper findLeft(Zipper zipper, java.util.function.Predicate<Zipper> pred) {
    IZipHandler handler =
        new BaseHandler() {
          @Override
          public Zipper onStepAtLeftMost(Zipper z) {
            return null;
          }
        };
    Zipper z =
        new Zipper(
            zipper.parent,
            zipper.left,
            zipper.right,
            zipper.depth,
            zipper.changed,
            zipper.context,
            handler);
    return find(z, Zip::stepLeft, pred);
  }

  public static Zipper findRight(Zipper zipper, java.util.function.Predicate<Zipper> pred) {
    IZipHandler handler =
        new BaseHandler() {
          @Override
          public Zipper onStepAtRightMost(Zipper z) {
            return null;
          }
        };
    Zipper z =
        new Zipper(
            zipper.parent,
            zipper.left,
            zipper.right,
            zipper.depth,
            zipper.changed,
            zipper.context,
            handler);
    return find(z, Zip::stepRight, pred);
  }

  public static Zipper prewalk(Zipper zipper, java.util.function.Function<Object, Object> f) {
    Zipper z = replace(zipper, f.apply(zipper.rightElement()));

    if (!z.atInsideMost() && !z.isEmptyContainer()) {
      Zipper current = stepInside(z);
      while (!current.atRightMost()) {
        current = stepRight(prewalk(current, f));
      }
      return stepOutside(current);
    } else {
      return z;
    }
  }

  public static Zipper postwalk(Zipper zipper, java.util.function.Function<Object, Object> f) {
    Zipper z = zipper;
    if (!z.atInsideMost() && !z.isEmptyContainer()) {
      Zipper current = stepInside(z);
      while (!current.atRightMost()) {
        current = stepRight(postwalk(current, f));
      }
      z = stepOutside(current);
    }

    return replace(z, f.apply(z.rightElement()));
  }
}
