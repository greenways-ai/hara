package hara.kernel.redirect;

import hara.lang.base.Ex;
import hara.kernel.builtin.BuiltinUtil;
import hara.kernel.protocol.IRedirect;
import hara.lang.data.Vector;
import hara.lang.protocol.IDisplay;

import java.io.*;
import java.util.Arrays;

public class FileRedirect implements IRedirect {

  private final PrintStream logIn;
  private final PrintStream logOut;

  public FileRedirect(String logInPath, String logOutPath) throws FileNotFoundException {
    if (logInPath != null) {
      File logInFile = new File(logInPath);
      File parentDir = logInFile.getParentFile();
      if (parentDir != null) {
        parentDir.mkdirs();
      }
      this.logIn = new PrintStream(new FileOutputStream(logInFile, true));
    } else {
      this.logIn = null;
    }

    if (logOutPath != null) {
      File logOutFile = new File(logOutPath);
      File parentDir = logOutFile.getParentFile();
      if (parentDir != null) {
        parentDir.mkdirs();
      }
      this.logOut = new PrintStream(new FileOutputStream(logOutFile, true));
    } else {
      this.logOut = null;
    }
  }

  @Override
  public void read(Object obj) throws IOException {
    if (this.logIn != null) {
      String output;
      if (obj instanceof java.util.List) {
        java.util.List<?> list = (java.util.List<?>) obj;
        Vector.Mutable<String> vec = Vector.Mutable.empty(null);
        for (Object item : list) {
          if (item instanceof byte[]) {
            StringBuilder sb = new StringBuilder();
            for (byte b : (byte[]) item) {
              sb.append(String.format("%02x", b));
            }
            vec.conj(sb.toString());
          } else {
            vec.conj(item.toString());
          }
        }
        output = BuiltinUtil.prStr(vec.toPersistent());
      } else {
        output = obj.toString();
      }
      this.logIn.println("COMMAND," + output);
      this.logIn.flush();
    }
  }

  @Override
  public void write(Object obj) throws IOException {
    if (this.logOut != null) {
      String output;
      if (obj instanceof IDisplay) {
        output = BuiltinUtil.prStr(obj);
      } else if (obj instanceof byte[]) {
        StringBuilder sb = new StringBuilder();
        for (byte b : (byte[]) obj) {
          sb.append(String.format("%02x", b));
        }
        output = sb.toString();
      } else if (obj instanceof Object[]) {
        output = Arrays.toString((Object[]) obj);
      } else if (obj instanceof Throwable) {
        this.logOut.println("RESPONSE," + ((Throwable) obj).getMessage());
        ((Throwable) obj).printStackTrace(this.logOut);
        this.logOut.flush();
        return;
      } else {
        output = obj.toString();
      }
      this.logOut.println("RESPONSE," + output);
      this.logOut.flush();
    }
  }

  @Override
  public void close() throws IOException {
    if (this.logIn != null) {
      this.logIn.close();
    }
    if (this.logOut != null) {
      this.logOut.close();
    }
  }
}
