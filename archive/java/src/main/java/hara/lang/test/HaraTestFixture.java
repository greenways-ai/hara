package hara.lang.test;

import hara.truffle.HaraFunction;

public final class HaraTestFixture {
  private final String namespace;
  private final String phase;
  private final HaraFunction body;

  public HaraTestFixture(String namespace, String phase, HaraFunction body) {
    this.namespace = namespace;
    this.phase = phase;
    this.body = body;
  }

  public String namespace() {
    return namespace;
  }

  public String phase() {
    return phase;
  }

  public HaraFunction body() {
    return body;
  }
}
