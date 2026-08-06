package hara.kernel;

import hara.lang.base.Ex;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

public final class NativeMode {

  public static final String PROPERTY = "hara.native";

  private NativeMode() {}

  public static boolean enabled() {
    return Boolean.parseBoolean(System.getProperty(PROPERTY, "false"));
  }

  public static void enable() {
    System.setProperty(PROPERTY, "true");
  }

  public static void requireDisabled(String feature) {
    if (enabled()) {
      throw unsupported(feature);
    }
  }

  public static Ex.Unsupported unsupported(String feature) {
    return new Ex.Unsupported(
        "Native mode does not support "
            + feature
            + ". Use the default JVM runtime for features that depend on dynamic runtime behavior.");
  }

  public static List<Class<?>> commandClasses() {
    // Native mode intentionally exposes only the statically linked command set.
    List<Class<?>> classes =
        new ArrayList<>(
            Arrays.asList(
                hara.kernel.command.Core.class,
                hara.kernel.command.Jvm.class,
                hara.kernel.command.Os.class,
                hara.kernel.command.Server.class,
                hara.kernel.command.Session.class,
                hara.kernel.command.Peer.class));
    if (!enabled()) {
      classes.add(hara.kernel.command.Maven.class);
    }
    return classes;
  }
}
