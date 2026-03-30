package l2s.dateditor.compiler;

import java.util.HashMap;
import java.util.Map;

public class MemoryClassLoader extends ClassLoader {
    private final Map<String, MemoryByteCode> classes = new HashMap<>();
    private final Map<String, MemoryByteCode> loaded = new HashMap<>();

    protected Class<?> findClass(String name) throws ClassNotFoundException {
        MemoryByteCode mbc = this.classes.get(name);
        if (mbc == null) {
            mbc = this.classes.get(name);
            if (mbc == null) {
                return super.findClass(name);
            }
        }

        return this.defineClass(name, mbc.getBytes(), 0, mbc.getBytes().length);
    }

    public void addClass(MemoryByteCode mbc) {
        this.classes.put(mbc.getName(), mbc);
        this.loaded.put(mbc.getName(), mbc);
    }

    public MemoryByteCode getClass(String name) {
        return this.classes.get(name);
    }

    public String[] getLoadedClasses() {
        return this.loaded.keySet().toArray(new String[0]);
    }

    public void clear() {
        this.loaded.clear();
    }
}
