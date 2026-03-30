package com.majestic.studio.compiler;

import org.apache.log4j.LogManager;
import org.apache.log4j.Logger;
import org.eclipse.jdt.internal.compiler.tool.EclipseCompiler;
import org.eclipse.jdt.internal.compiler.tool.EclipseFileManager;

import javax.tools.*;
import java.io.File;
import java.io.StringWriter;
import java.io.Writer;
import java.nio.charset.Charset;
import java.util.ArrayList;
import java.util.Collection;
import java.util.List;
import java.util.Locale;

public class Compiler {
    private static final Logger LOGGER = LogManager.getLogger(Compiler.class);
    private static final JavaCompiler javac = new EclipseCompiler();
    private final DiagnosticListener<JavaFileObject> listener = new DefaultDiagnosticListener();
    private final StandardJavaFileManager fileManager = new EclipseFileManager(Locale.getDefault(), Charset.defaultCharset());
    private final MemoryClassLoader memClassLoader = new MemoryClassLoader();
    private final MemoryJavaFileManager memFileManager;

    public Compiler() {
        this.memFileManager = new MemoryJavaFileManager(this.fileManager, this.memClassLoader);
    }

    public boolean compile(File... files) {
        List<String> options = new ArrayList<>();
        options.add("-Xlint:all");
        options.add("-warn:none");
        options.add("-g");
        options.add("-1.8");
        Writer writer = new StringWriter();
        JavaCompiler.CompilationTask compile = javac.getTask(writer, this.memFileManager, this.listener, options, null, this.fileManager.getJavaFileObjects(files));
        return compile.call();
    }

    public boolean compile(Collection<File> files) {
        return this.compile(files.toArray(new File[0]));
    }

    public MemoryClassLoader getClassLoader() {
        return this.memClassLoader;
    }

    private static class DefaultDiagnosticListener implements DiagnosticListener<JavaFileObject> {
        private DefaultDiagnosticListener() {
        }

        public void report(Diagnostic<? extends JavaFileObject> diagnostic) {
            Compiler.LOGGER.error(diagnostic.getSource().getName() + (diagnostic.getPosition() == -1L ? "" : ":" + diagnostic.getLineNumber() + "," + diagnostic.getColumnNumber()) + ": " + diagnostic.getMessage(Locale.getDefault()));
        }
    }
}
