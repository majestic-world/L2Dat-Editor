package l2s.dateditor.compiler;

import javax.tools.*;
import java.io.IOException;
import java.net.URI;

public class MemoryJavaFileManager extends ForwardingJavaFileManager<StandardJavaFileManager> {
    private final MemoryClassLoader cl;

    public MemoryJavaFileManager(StandardJavaFileManager sjfm, MemoryClassLoader xcl) {
        super(sjfm);
        this.cl = xcl;
    }

    public JavaFileObject getJavaFileForOutput(JavaFileManager.Location location, String className, JavaFileObject.Kind kind, FileObject sibling) throws IOException {
        MemoryByteCode mbc = new MemoryByteCode(className.replace('/', '.').replace('\\', '.'), URI.create("file:///" + className.replace('.', '/').replace('\\', '/') + kind.extension));
        this.cl.addClass(mbc);
        return mbc;
    }

    public ClassLoader getClassLoader(JavaFileManager.Location location) {
        return this.cl;
    }
}
