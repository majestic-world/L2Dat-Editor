package jfork.nproperty;

public interface IPropertyListener {
   void onStart(String var1);

   void onPropertyMiss(String var1);

   void onDone(String var1);

   void onInvalidPropertyCast(String var1, String var2);
}
