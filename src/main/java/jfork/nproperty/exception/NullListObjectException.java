package jfork.nproperty.exception;

import java.io.IOException;

public class NullListObjectException extends IOException {
   public NullListObjectException() {
   }

   public NullListObjectException(String message) {
      super(message);
   }

   public NullListObjectException(String message, Throwable cause) {
      super(message, cause);
   }

   public NullListObjectException(Throwable cause) {
      super(cause);
   }
}
