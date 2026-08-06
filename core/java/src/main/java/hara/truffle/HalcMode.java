package hara.truffle;

enum HalcMode {
  AUTO,
  STRICT,
  OFF;

  static HalcMode current() {
    String value =
        System.getProperty(
            "hara.HalcMode", System.getProperty("hara.HirMode", "auto"));
    return switch (value.toLowerCase(java.util.Locale.ROOT)) {
      case "auto" -> AUTO;
      case "strict" -> STRICT;
      case "off" -> OFF;
      default -> throw new HaraException(
          "hara.HalcMode expects auto, strict, or off; received " + value);
    };
  }
}
