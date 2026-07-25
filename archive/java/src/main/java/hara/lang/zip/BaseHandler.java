package hara.lang.zip;

public class BaseHandler implements IZipHandler {

  @Override
  public Zipper onStepAtLeftMost(Zipper zipper) {
    return zipper;
  }

  @Override
  public Zipper onStepAtRightMost(Zipper zipper) {
    return zipper;
  }

  @Override
  public Zipper onStepAtInsideMost(Zipper zipper) {
    return zipper;
  }

  @Override
  public Zipper onStepAtInsideMostLeft(Zipper zipper) {
    return zipper;
  }

  @Override
  public Zipper onStepAtOutsideMost(Zipper zipper) {
    return zipper;
  }

  @Override
  public Zipper onDeleteAtLeftMost(Zipper zipper) {
    return zipper;
  }

  @Override
  public Zipper onDeleteAtRightMost(Zipper zipper) {
    return zipper;
  }
}
