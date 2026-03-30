package l2s.dateditor.actions;

import l2s.dateditor.Boot;
import l2s.dateditor.config.ConfigWindow;

import java.io.File;
import java.io.FileOutputStream;
import java.io.PrintWriter;
import java.util.logging.Level;
import java.util.logging.Logger;

public class ExportTxt extends ActionTask {
    private final File file;

    public ExportTxt(Boot boot, File file) {
        super(boot);
        this.file = file;
    }

    protected void action() {
        try {
            if (this.isCancelled()) {
                return;
            }

            this.changeProgress(15.0F);
            PrintWriter out = new PrintWriter(new FileOutputStream(this.file.getPath()), true);
            this.changeProgress(30.0F);
            ConfigWindow.save("FILE_SAVE_CURRENT_DIRECTORY", this.file.getParentFile().toString());
            this.changeProgress(50.0F);
            out.print(this.boot.getTextPaneMain().getText());
            this.changeProgress(90.0F);
            out.close();
        } catch (Exception ex) {
            Logger.getLogger(Boot.class.getName()).log(Level.SEVERE, null, ex);
        }

        Boot.addLogConsole("---------------------------------------", true);
        Boot.addLogConsole("Saved: " + this.file.getPath(), true);
    }
}
