package jfork.typecaster.exception;

import java.io.IOException;

public class IllegalTypeException extends IOException {
   public IllegalTypeException() {
   }

   public IllegalTypeException(String message) {
      super(message);
   }

   public IllegalTypeException(String message, Throwable cause) {
      super(message, cause);
   }

   public IllegalTypeException(Throwable cause) {
      super(cause);
   }
}
