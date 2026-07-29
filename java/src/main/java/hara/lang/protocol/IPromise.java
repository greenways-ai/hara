package hara.lang.protocol;

/** An asynchronous value with composable settlement handlers. */
public interface IPromise
    extends IDeref<Object>, IDerefTimeout<Object> {
  Object state();

  Object value();

  Object then(Object function);

  Object catchError(Object function);

  Object finallyDo(Object function);

  Object cancel();
}
