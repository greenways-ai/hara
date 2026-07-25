package hara.lang.test;

import hara.truffle.HaraFunction;

public final class HaraTestCase {
  private final String namespace;
  private final String name;
  private final Object metadata;
  private final HaraFunction body;

  public HaraTestCase(String namespace, String name, Object metadata, HaraFunction body) {
    this.namespace = namespace;
    this.name = name;
    this.metadata = metadata;
    this.body = body;
  }

  public String namespace() {
    return namespace;
  }

  public String name() {
    return name;
  }

  public Object metadata() {
    return metadata;
  }

  public HaraFunction body() {
    return body;
  }
}
