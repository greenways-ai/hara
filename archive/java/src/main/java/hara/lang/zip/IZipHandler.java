package hara.lang.zip;

public interface IZipHandler {
  Zipper onStepAtLeftMost(Zipper zipper);

  Zipper onStepAtRightMost(Zipper zipper);

  Zipper onStepAtInsideMost(Zipper zipper);

  Zipper onStepAtInsideMostLeft(Zipper zipper);

  Zipper onStepAtOutsideMost(Zipper zipper);

  Zipper onDeleteAtLeftMost(Zipper zipper);

  Zipper onDeleteAtRightMost(Zipper zipper);
}
