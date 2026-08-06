package hara.kernel.maven;

import hara.kernel.protocol.IRuntime;
import hara.lang.base.Ex;
import hara.lang.protocol.IOFn;
import org.eclipse.aether.resolution.DependencyResolutionException;

import java.io.File;
import java.net.MalformedURLException;
import java.util.List;
import java.util.function.BiFunction;
import java.util.function.Function;

public final class Maven {

  private static final MavenResolver resolver = new MavenResolver();

  private static Object doLoad(IRuntime rt, String coordinate) {
    try {
      List<File> files = resolver.resolve(coordinate);
      for (File file : files) {
        rt.pathAdd(file.toURI().toURL());
      }
      return true;
    } catch (DependencyResolutionException | MalformedURLException e) {
      throw Ex.Sneaky(e);
    }
  }

  public static final IOFn load =
      new IOFn() {
        @Override
        public BiFunction<Object, Object, Object> getArg2() {
          return (rt, coordinate) -> doLoad((IRuntime) rt, (String) coordinate);
        }

        @Override
        public Function<Object, Object> getArgN() {
          return (vargs) -> {
            Object[] args = (Object[]) vargs;
            return doLoad((IRuntime) args[0], (String) args[1]);
          };
        }
      };
}
