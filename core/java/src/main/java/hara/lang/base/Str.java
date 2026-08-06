package hara.lang.base;

import hara.lang.string.Escape;

public interface Str {

  /**
   * Used to invert an escape array into an unescape array
   *
   * @param array String[][] to be inverted
   * @return String[][] inverted array
   */
  public static String[][] invert(final String[][] array) {
    final String[][] newarray = new String[array.length][2];
    for (int i = 0; i < array.length; i++) {
      newarray[i][0] = array[i][1];
      newarray[i][1] = array[i][0];
    }
    return newarray;
  }

  public static String escapeJava(final String input) {
    return Escape.ESCAPE_JAVA.translate(input);
  }
}
