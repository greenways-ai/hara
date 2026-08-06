package hara.kernel.base;

import org.jline.reader.LineReader;

import java.io.IOException;
import java.io.Reader;
import java.util.function.Supplier;

public class JLineInputReader extends Reader {
  public enum CommandResult {
    PASS,
    CONSUMED,
    EOF
  }

  @FunctionalInterface
  public interface CommandHandler {
    CommandResult handle(String line);
  }

  private final LineReader lineReader;
  private final Supplier<String> primaryPrompt;
  private final Supplier<String> continuationPrompt;
  private final CommandHandler commandHandler;
  private String buffer = "";
  private int pos = 0;
  private boolean firstLine = true;

  public JLineInputReader(LineReader lineReader) {
    this(lineReader, () -> "> ", () -> "... ", line -> CommandResult.PASS);
  }

  public JLineInputReader(
      LineReader lineReader,
      Supplier<String> primaryPrompt,
      Supplier<String> continuationPrompt,
      CommandHandler commandHandler) {
    this.lineReader = lineReader;
    this.primaryPrompt = primaryPrompt;
    this.continuationPrompt = continuationPrompt;
    this.commandHandler = commandHandler;
  }

  @Override
  public int read(char[] cbuf, int off, int len) throws IOException {
    if (len == 0) return 0;

    ensureBuffer();

    if (buffer == null) return -1; // EOF

    int count = 0;
    while (count < len) {
      if (pos >= buffer.length()) {
        ensureBuffer();
        if (buffer == null) {
          return (count == 0) ? -1 : count;
        }
      }

      int toCopy = Math.min(len - count, buffer.length() - pos);
      buffer.getChars(pos, pos + toCopy, cbuf, off + count);
      pos += toCopy;
      count += toCopy;
    }
    return count;
  }

  private void ensureBuffer() {
    if (buffer != null && pos >= buffer.length()) {
      while (true) {
        try {
          String prompt = firstLine ? primaryPrompt.get() : continuationPrompt.get();
          String line = lineReader.readLine(prompt);
          if (line == null) {
            buffer = null;
            return;
          }
          if (firstLine && line.startsWith("/")) {
            CommandResult result = commandHandler.handle(line);
            if (result == CommandResult.EOF) {
              buffer = null;
              return;
            }
            if (result == CommandResult.CONSUMED) {
              continue;
            }
          }
          buffer = line + "\n";
          pos = 0;
          firstLine = false;
          return;
        } catch (org.jline.reader.UserInterruptException e) {
          buffer = "\n";
          pos = 0;
          firstLine = true;
          return;
        } catch (org.jline.reader.EndOfFileException e) {
          buffer = null;
          return;
        }
      }
    }
  }

  public void resetPrompt() {
    firstLine = true;
  }

  @Override
  public void close() throws IOException {
    // No-op
  }
}
