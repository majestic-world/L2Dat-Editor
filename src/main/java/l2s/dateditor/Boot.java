package l2s.dateditor;

import l2s.dateditor.actions.*;
import l2s.dateditor.clientcryptor.crypt.DatCrypter;
import l2s.dateditor.config.ConfigDebug;
import l2s.dateditor.config.ConfigWindow;
import l2s.dateditor.forms.JPopupTextArea;
import l2s.dateditor.util.DebugUtil;
import l2s.dateditor.xml.CryptVersionParser;
import l2s.dateditor.xml.DescriptorParser;
import org.apache.commons.lang3.ArrayUtils;
import org.apache.commons.lang3.StringUtils;
import org.fusesource.jansi.Ansi;
import org.fusesource.jansi.AnsiConsole;

import javax.swing.*;
import javax.swing.event.DocumentEvent;
import javax.swing.event.DocumentListener;
import javax.swing.filechooser.FileNameExtensionFilter;
import javax.swing.text.AbstractDocument;
import javax.swing.text.AttributeSet;
import javax.swing.text.BadLocationException;
import javax.swing.text.DocumentFilter;
import java.awt.*;
import java.awt.event.ActionEvent;
import java.awt.event.WindowAdapter;
import java.awt.event.WindowEvent;
import java.io.File;
import java.util.ArrayList;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.logging.Level;
import java.util.logging.Logger;

public class Boot extends JFrame {
    public static boolean DEV_MODE = false;
    public static boolean MASS_FULL_LOG = false;
    public static boolean INCLUDE_ARENA = false;
    private final ExecutorService executorService = Executors.newCachedThreadPool();
    private static SplashScreen splashScreen = null;
    private static JTextArea textPaneLog;
    private final JPopupTextArea textPaneMain;
    private final LineNumberingTextArea lineNumberingTextArea;
    private final JComboBox<String> jComboBoxStructure;
    private final JComboBox<String> jComboBoxEncrypt;
    private final JComboBox<String> jComboBoxFormatter;
    private final JComboBox<String> jComboBoxEnum;
    private final ArrayList<JPanel> actionPanels = new ArrayList<>();
    private final JPanel settingsPanel = new JPanel();
    private final JPanel buttonsPanel = new JPanel();
    private final JButton openButton;
    private final JButton exportTxtButton;
    private final JButton saveButton;
    private final JButton abortTaskButton;
    private final JProgressBar progressBar;
    private File currentFileWindow = null;
    private ActionTask progressTask = null;

    public static void main(String[] args) {
        if (System.console() != null) {
            AnsiConsole.systemInstall();
        }

        splashScreen = SplashScreen.getSplashScreen();
        DEV_MODE = ArrayUtils.contains(args, "-dev");
        MASS_FULL_LOG = ArrayUtils.contains(args, "-mass_full_log");
        INCLUDE_ARENA = ArrayUtils.contains(args, "-include_arena");
        ConfigWindow.load();
        ConfigDebug.load();
        CryptVersionParser.getInstance().parse();
        DescriptorParser.getInstance().parse();

        try {
            for (UIManager.LookAndFeelInfo info : UIManager.getInstalledLookAndFeels()) {
                if ("Nimbus".equals(info.getName())) {
                    UIManager.setLookAndFeel(info.getClassName());
                    break;
                }
            }
        } catch (Exception ex) {
            Logger.getLogger(Boot.class.getName()).log(Level.SEVERE, null, ex);
        }

        EventQueue.invokeLater(Boot::new);
    }

    public Boot() {
        this.setTitle("Lineage 2 Editor By Mk v" + this.getClass().getPackage().getImplementationVersion());
        this.setMinimumSize(new Dimension(1000, 600));
        this.setSize(new Dimension(ConfigWindow.WINDOW_WIDTH, ConfigWindow.WINDOW_HEIGHT));
        this.getContentPane().setLayout(new BorderLayout());
        this.setDefaultCloseOperation(WindowConstants.EXIT_ON_CLOSE);
        this.setLocationRelativeTo(null);
        this.addWindowListener(new WindowAdapter() {
            public void windowClosing(WindowEvent evt) {
                ConfigWindow.save("WINDOW_HEIGHT", String.valueOf(Boot.this.getHeight()));
                ConfigWindow.save("WINDOW_WIDTH", String.valueOf(Boot.this.getWidth()));
                System.exit(0);
            }
        });
        JPanel buttonPane = new JPanel();
        buttonPane.setLayout(new BorderLayout());
        JLabel structureLabel = new JLabel("Structures:");
        this.settingsPanel.add(structureLabel);
        this.jComboBoxStructure = new JComboBox<>();
        String[] chronicles = DescriptorParser.getInstance().getChronicleNames().toArray(new String[0]);
        this.jComboBoxStructure.setModel(new DefaultComboBoxModel<>(chronicles));
        this.jComboBoxStructure.setSelectedItem(!ArrayUtils.contains(chronicles, ConfigWindow.CURRENT_STRUCTURE) ? chronicles[chronicles.length - 1] : ConfigWindow.CURRENT_STRUCTURE);
        ConfigWindow.CURRENT_STRUCTURE = String.valueOf(this.jComboBoxStructure.getSelectedItem());
        this.jComboBoxStructure.addActionListener((e) -> this.saveComboBox(this.jComboBoxStructure, "CURRENT_STRUCTURE"));
        this.settingsPanel.add(this.jComboBoxStructure);
        JLabel encryptLabel = new JLabel("Encrypt:");
        this.settingsPanel.add(encryptLabel);
        this.jComboBoxEncrypt = new JComboBox<>();
        DefaultComboBoxModel<String> comboBoxModel = new DefaultComboBoxModel<>(CryptVersionParser.getInstance().getEncryptKey().keySet().toArray(new String[0]));
        comboBoxModel.insertElementAt("Source", 0);
        comboBoxModel.setSelectedItem("Source");
        this.jComboBoxEncrypt.setModel(comboBoxModel);
        this.jComboBoxEncrypt.setSelectedItem(ConfigWindow.CURRENT_ENCRYPT);
        this.jComboBoxEncrypt.addActionListener((e) -> this.saveComboBox(this.jComboBoxEncrypt, "CURRENT_ENCRYPT"));
        this.settingsPanel.add(this.jComboBoxEncrypt);
        buttonPane.add(this.settingsPanel, "First");
        this.actionPanels.add(this.settingsPanel);
        JLabel inputFormatterLabel = new JLabel("Formatter:");
        this.settingsPanel.add(inputFormatterLabel);
        this.jComboBoxFormatter = new JComboBox<>();
        comboBoxModel = new DefaultComboBoxModel<>(new String[]{"Enabled", "Disabled"});
        this.jComboBoxFormatter.setModel(comboBoxModel);
        this.jComboBoxFormatter.setSelectedItem(ConfigWindow.CURRENT_FORMATTER);
        this.jComboBoxFormatter.addActionListener((e) -> this.saveComboBox(this.jComboBoxFormatter, "CURRENT_FORMATTER"));
        this.settingsPanel.add(this.jComboBoxFormatter);
        JLabel inputEnumLabel = new JLabel("Enums:");
        this.settingsPanel.add(inputEnumLabel);
        this.jComboBoxEnum = new JComboBox<>();
        comboBoxModel = new DefaultComboBoxModel<>(new String[]{"Enabled", "Disabled"});
        this.jComboBoxEnum.setModel(comboBoxModel);
        this.jComboBoxEnum.setSelectedItem(ConfigWindow.CURRENT_ENUM);
        this.jComboBoxEnum.addActionListener((e) -> this.saveComboBox(this.jComboBoxEnum, "CURRENT_ENUM"));
        this.settingsPanel.add(this.jComboBoxEnum);
        this.openButton = new JButton();
        this.openButton.setText("Open");
        this.openButton.addActionListener(this::openSelectFileWindow);
        this.buttonsPanel.add(this.openButton);
        this.saveButton = new JButton();
        this.saveButton.setText("Save");
        this.saveButton.addActionListener(this::saveActionPerformed);
        this.saveButton.setEnabled(false);
        this.buttonsPanel.add(this.saveButton);
        this.exportTxtButton = new JButton();
        this.exportTxtButton.setText("Export to TXT");
        this.exportTxtButton.addActionListener(this::exportTxtActionPerformed);
        this.exportTxtButton.setEnabled(false);
        this.buttonsPanel.add(this.exportTxtButton);
        JSeparator separator = new JSeparator(1);
        separator.setPreferredSize(new Dimension(2, 20));
        this.buttonsPanel.add(separator);
        separator = new JSeparator(1);
        separator.setPreferredSize(new Dimension(2, 20));
        this.buttonsPanel.add(separator);
        JButton massTxtUnpack = new JButton();
        massTxtUnpack.setText("Unpack");
        massTxtUnpack.addActionListener(this::unpackActionPerformed);
        this.buttonsPanel.add(massTxtUnpack);
        JButton massTxtPack = new JButton();
        massTxtPack.setText("Pack");
        massTxtPack.addActionListener(this::massPackActionPerformed);
        this.buttonsPanel.add(massTxtPack);
        separator = new JSeparator(1);
        separator.setPreferredSize(new Dimension(2, 20));
        this.buttonsPanel.add(separator);
        separator = new JSeparator(1);
        separator.setPreferredSize(new Dimension(2, 20));
        this.buttonsPanel.add(separator);
        JButton massRecrypt = new JButton();
        massRecrypt.setText("Change crypt");
        massRecrypt.addActionListener(this::changeCryptActionPerformed);
        this.buttonsPanel.add(massRecrypt);
        buttonPane.add(this.buttonsPanel);
        this.actionPanels.add(this.buttonsPanel);
        JPanel progressPane = new JPanel();
        this.progressBar = new JProgressBar(0, 100);
        this.progressBar.setPreferredSize(new Dimension(300, 25));
        this.progressBar.setValue(0);
        this.progressBar.setStringPainted(true);
        progressPane.add(this.progressBar);
        this.abortTaskButton = new JButton();
        this.abortTaskButton.setText("Abort");
        this.abortTaskButton.addActionListener(this::abortActionPerformed);
        this.abortTaskButton.setEnabled(false);
        progressPane.add(this.abortTaskButton);
        buttonPane.add(progressPane, "Last");
        JSplitPane jsp = new JSplitPane(JSplitPane.VERTICAL_SPLIT, false);
        jsp.setResizeWeight(0.7);
        jsp.setOneTouchExpandable(true);
        Font font = new Font((new JLabel()).getFont().getName(), Font.BOLD, 13);
        this.textPaneMain = new JPopupTextArea();
        this.textPaneMain.setBackground(new Color(41, 49, 52));
        this.textPaneMain.setForeground(Color.WHITE);
        this.textPaneMain.setFont(font);
        ((AbstractDocument) this.textPaneMain.getDocument()).setDocumentFilter(new DocumentFilter() {
            public void replace(DocumentFilter.FilterBypass fb, int offset, int length, String text, AttributeSet attrs) throws BadLocationException {
                text = text.replace("\r\n", "\n");
                super.replace(fb, offset, length, text, attrs);
            }
        });
        this.lineNumberingTextArea = new LineNumberingTextArea(this.textPaneMain);
        this.lineNumberingTextArea.setBackground(Color.DARK_GRAY);
        this.lineNumberingTextArea.setForeground(Color.LIGHT_GRAY);
        this.lineNumberingTextArea.setFont(font.deriveFont(12.0F));
        this.lineNumberingTextArea.setEditable(false);
        this.textPaneMain.getDocument().addDocumentListener(this.lineNumberingTextArea);
        JScrollPane jScrollPane1 = new JScrollPane(ScrollPaneConstants.VERTICAL_SCROLLBAR_ALWAYS, ScrollPaneConstants.HORIZONTAL_SCROLLBAR_AS_NEEDED);
        jScrollPane1.setAutoscrolls(true);
        jScrollPane1.setViewportView(this.textPaneMain);
        jScrollPane1.setRowHeaderView(this.lineNumberingTextArea);
        jsp.setTopComponent(jScrollPane1);
        textPaneLog = new JPopupTextArea();
        textPaneLog.setBackground(new Color(41, 49, 52));
        textPaneLog.setForeground(Color.CYAN);
        textPaneLog.setEditable(false);
        JScrollPane jScrollPane2 = new JScrollPane();
        jScrollPane2.setViewportView(textPaneLog);
        jScrollPane2.setAutoscrolls(true);
        jsp.setBottomComponent(jScrollPane2);
        this.getContentPane().add(buttonPane, "First");
        this.getContentPane().add(jsp);
        this.pack();
        if (splashScreen != null) {
            splashScreen.close();
        }

        this.setVisible(true);
        this.toFront();
    }

    public JPopupTextArea getTextPaneMain() {
        return this.textPaneMain;
    }

    public static void addLogConsole(String log, boolean isLog) {
        if (isLog) {
            DebugUtil.getLogger().info(Ansi.ansi().fgCyan().a(log).reset());
        }

        if (!SwingUtilities.isEventDispatchThread()) {
            SwingUtilities.invokeLater(() -> textPaneLog.append(log + "\n"));
        } else {
            textPaneLog.append(log + "\n");
        }

    }

    public static void addErrorConsole(String log, boolean isLog) {
        if (isLog) {
            DebugUtil.getLogger().info(Ansi.ansi().fgRed().a(log).reset());
        }

        if (!SwingUtilities.isEventDispatchThread()) {
            SwingUtilities.invokeLater(() -> textPaneLog.append(log + "\n"));
        } else {
            textPaneLog.append(log + "\n");
        }

    }

    public void setEditorText(String text) {
        this.lineNumberingTextArea.cleanUp();
        if (!SwingUtilities.isEventDispatchThread()) {
            SwingUtilities.invokeLater(() -> this.textPaneMain.setText(text));
        } else {
            this.textPaneMain.setText(text);
        }

    }

    private void massPackActionPerformed(ActionEvent evt) {
        if (this.progressTask == null) {
            JFileChooser fileopen = new JFileChooser();
            fileopen.setFileSelectionMode(JFileChooser.FILES_AND_DIRECTORIES);
            fileopen.setMultiSelectionEnabled(true);
            fileopen.setAcceptAllFileFilterUsed(false);
            fileopen.setFileFilter(new FileNameExtensionFilter(".ini", "ini"));
            fileopen.setFileFilter(new FileNameExtensionFilter(".txt", "txt"));
            fileopen.setFileFilter(new FileNameExtensionFilter(".htm", "htm"));
            fileopen.setFileFilter(new FileNameExtensionFilter(".ini, .txt, .htm", "ini", "txt", "htm"));
            if (ConfigWindow.OUTPUT_DIRECTORY.equalsIgnoreCase(".")) {
                fileopen.setCurrentDirectory(new File(ConfigWindow.INPUT_DIRECTORY));
            } else {
                fileopen.setCurrentDirectory(new File(ConfigWindow.OUTPUT_DIRECTORY));
            }

            fileopen.setPreferredSize(new Dimension(600, 600));
            fileopen.setDialogTitle("Select directories and files for packing.");
            int ret = fileopen.showDialog(null, "Select");
            if (ret == 0) {
                File[] selectedInputFiles = fileopen.getSelectedFiles();
                if (selectedInputFiles == null || selectedInputFiles.length == 0) {
                    addErrorConsole("Files not selected for unpack.", true);
                    return;
                }

                String path = selectedInputFiles[0].getParent();
                if (selectedInputFiles[0].isDirectory() && selectedInputFiles.length == 1) {
                    path = selectedInputFiles[0].getPath();
                    selectedInputFiles = selectedInputFiles[0].listFiles();
                    if (selectedInputFiles == null) {
                        selectedInputFiles = new File[0];
                    }
                }

                ConfigWindow.save("OUTPUT_DIRECTORY", path);
                addLogConsole("---------------------------------------", true);
                addLogConsole("Selected folder: " + path, true);
                fileopen = new JFileChooser();
                fileopen.setFileSelectionMode(JFileChooser.DIRECTORIES_ONLY);
                fileopen.setAcceptAllFileFilterUsed(false);
                if (ConfigWindow.INPUT_DIRECTORY.equalsIgnoreCase(".")) {
                    fileopen.setCurrentDirectory(new File(ConfigWindow.OUTPUT_DIRECTORY));
                } else {
                    fileopen.setCurrentDirectory(new File(ConfigWindow.INPUT_DIRECTORY));
                }

                fileopen.setPreferredSize(new Dimension(600, 600));
                fileopen.setDialogTitle("Select packed files output directory.");
                ret = fileopen.showDialog(null, "Select");
                if (ret == 0) {
                    File outputDir = fileopen.getSelectedFile();
                    if (outputDir == null) {
                        addErrorConsole("Selected non-exist output directory.", true);
                        return;
                    }

                    ConfigWindow.save("INPUT_DIRECTORY", outputDir.getPath());
                    this.progressTask = new FilePacker(this, selectedInputFiles, outputDir);
                    this.executorService.execute(this.progressTask);
                }
            }

        }
    }

    private void unpackActionPerformed(ActionEvent evt) {
        if (this.progressTask == null) {
            JFileChooser fileopen = new JFileChooser();
            fileopen.setFileSelectionMode(JFileChooser.FILES_AND_DIRECTORIES);
            fileopen.setMultiSelectionEnabled(true);
            fileopen.setAcceptAllFileFilterUsed(false);
            fileopen.setFileFilter(new FileNameExtensionFilter(".ini", "ini"));
            fileopen.setFileFilter(new FileNameExtensionFilter(".dat", "dat"));
            fileopen.setFileFilter(new FileNameExtensionFilter(".htm", "htm"));
            fileopen.setFileFilter(new FileNameExtensionFilter(".ini, .dat, .htm", "ini", "dat", "htm"));
            if (ConfigWindow.INPUT_DIRECTORY.equalsIgnoreCase(".")) {
                fileopen.setCurrentDirectory(new File(ConfigWindow.OUTPUT_DIRECTORY));
            } else {
                fileopen.setCurrentDirectory(new File(ConfigWindow.INPUT_DIRECTORY));
            }

            fileopen.setPreferredSize(new Dimension(600, 600));
            fileopen.setDialogTitle("Select directories and files for unpacking.");
            int ret = fileopen.showDialog(null, "Select");
            if (ret == 0) {
                File[] selectedInputFiles = fileopen.getSelectedFiles();
                if (selectedInputFiles == null || selectedInputFiles.length == 0) {
                    addErrorConsole("Files not selected for unpack.", true);
                    return;
                }

                String path = selectedInputFiles[0].getParent();
                if (selectedInputFiles[0].isDirectory() && selectedInputFiles.length == 1) {
                    path = selectedInputFiles[0].getPath();
                    selectedInputFiles = selectedInputFiles[0].listFiles();
                    if (selectedInputFiles == null) {
                        selectedInputFiles = new File[0];
                    }
                }

                ConfigWindow.save("INPUT_DIRECTORY", path);
                addLogConsole("---------------------------------------", true);
                addLogConsole("Selected folder: " + path, true);
                fileopen = new JFileChooser();
                fileopen.setFileSelectionMode(JFileChooser.DIRECTORIES_ONLY);
                fileopen.setAcceptAllFileFilterUsed(false);
                if (ConfigWindow.OUTPUT_DIRECTORY.equalsIgnoreCase(".")) {
                    fileopen.setCurrentDirectory(new File(ConfigWindow.INPUT_DIRECTORY));
                } else {
                    fileopen.setCurrentDirectory(new File(ConfigWindow.OUTPUT_DIRECTORY));
                }

                fileopen.setPreferredSize(new Dimension(600, 600));
                fileopen.setDialogTitle("Select unpacked files output directory.");
                ret = fileopen.showDialog(null, "Select");
                if (ret == 0) {
                    File outputDir = fileopen.getSelectedFile();
                    if (outputDir == null) {
                        addErrorConsole("Selected non-exist output directory.", true);
                        return;
                    }

                    ConfigWindow.save("OUTPUT_DIRECTORY", outputDir.getPath());
                    this.progressTask = new FileUnpacker(this, selectedInputFiles, outputDir);
                    this.executorService.execute(this.progressTask);
                }
            }

        }
    }

    private void changeCryptActionPerformed(ActionEvent evt) {
        if (this.progressTask == null) {
            JFileChooser fileopen = new JFileChooser();
            fileopen.setFileSelectionMode(JFileChooser.FILES_AND_DIRECTORIES);
            fileopen.setMultiSelectionEnabled(true);
            fileopen.setAcceptAllFileFilterUsed(false);
            fileopen.setFileFilter(new FileNameExtensionFilter(".ini", "ini"));
            fileopen.setFileFilter(new FileNameExtensionFilter(".dat", "dat"));
            fileopen.setFileFilter(new FileNameExtensionFilter(".htm", "htm"));
            fileopen.setFileFilter(new FileNameExtensionFilter(".ini, .dat, .htm", "ini", "dat", "htm"));
            if (ConfigWindow.INPUT_DIRECTORY.equalsIgnoreCase(".")) {
                fileopen.setCurrentDirectory(new File(ConfigWindow.OUTPUT_DIRECTORY));
            } else {
                fileopen.setCurrentDirectory(new File(ConfigWindow.INPUT_DIRECTORY));
            }

            fileopen.setPreferredSize(new Dimension(600, 600));
            fileopen.setDialogTitle("Select directories and files for change crypt.");
            int ret = fileopen.showDialog(null, "Select");
            if (ret == 0) {
                File[] selectedInputFiles = fileopen.getSelectedFiles();
                if (selectedInputFiles == null || selectedInputFiles.length == 0) {
                    addErrorConsole("Files not selected for change crypt.", true);
                    return;
                }

                String path = selectedInputFiles[0].getParent();
                if (selectedInputFiles[0].isDirectory() && selectedInputFiles.length == 1) {
                    path = selectedInputFiles[0].getPath();
                    selectedInputFiles = selectedInputFiles[0].listFiles();
                    if (selectedInputFiles == null) {
                        selectedInputFiles = new File[0];
                    }
                }

                ConfigWindow.save("INPUT_DIRECTORY", path);
                addLogConsole("---------------------------------------", true);
                addLogConsole("Selected folder: " + path, true);
                this.progressTask = new FileChangeCryptor(this, selectedInputFiles, path);
                this.executorService.execute(this.progressTask);
            }

        }
    }

    private void openSelectFileWindow(ActionEvent evt) {
        if (this.progressTask == null) {
            JFileChooser fileopen = new JFileChooser();
            fileopen.setFileSelectionMode(JFileChooser.FILES_ONLY);
            fileopen.setMultiSelectionEnabled(false);
            fileopen.setAcceptAllFileFilterUsed(false);
            fileopen.setFileFilter(new FileNameExtensionFilter(".dat", "dat"));
            fileopen.setFileFilter(new FileNameExtensionFilter(".ini", "ini"));
            fileopen.setFileFilter(new FileNameExtensionFilter(".txt", "txt"));
            fileopen.setFileFilter(new FileNameExtensionFilter(".htm", "htm"));
            fileopen.setFileFilter(new FileNameExtensionFilter(".dat, .ini, .txt, .htm", "dat", "ini", "txt", "htm"));
            if (ConfigWindow.INPUT_DIRECTORY.equalsIgnoreCase(".")) {
                fileopen.setCurrentDirectory(new File(ConfigWindow.OUTPUT_DIRECTORY));
            } else {
                fileopen.setCurrentDirectory(new File(ConfigWindow.INPUT_DIRECTORY));
            }

            if (!ConfigWindow.LAST_FILE_SELECTED.equalsIgnoreCase(".")) {
                fileopen.setSelectedFile(new File(ConfigWindow.LAST_FILE_SELECTED));
            }

            fileopen.setPreferredSize(new Dimension(600, 600));
            fileopen.setDialogTitle("Select file for open.");
            int ret = fileopen.showDialog(null, "Select");
            if (ret == 0) {
                this.currentFileWindow = fileopen.getSelectedFile();
                if (this.currentFileWindow == null || this.currentFileWindow.isDirectory()) {
                    addErrorConsole("Files not selected for change crypt.", true);
                    return;
                }

                ConfigWindow.save("LAST_FILE_SELECTED", this.currentFileWindow.getAbsolutePath());
                ConfigWindow.save("INPUT_DIRECTORY", this.currentFileWindow.getParent());
                addLogConsole("---------------------------------------", true);
                addLogConsole("Open file: " + this.currentFileWindow.getName(), true);
                this.progressTask = new OpenDat(this, this.currentFileWindow);
                this.executorService.execute(this.progressTask);
            }

        }
    }

    private void exportTxtActionPerformed(ActionEvent evt) {
        if (this.progressTask == null) {
            File file = this.currentFileWindow;
            if (file != null) {
                JFileChooser fileSave = new JFileChooser();
                fileSave.setFileSelectionMode(JFileChooser.FILES_ONLY);
                fileSave.setMultiSelectionEnabled(false);
                fileSave.setAcceptAllFileFilterUsed(false);
                if (ConfigWindow.OUTPUT_DIRECTORY.equalsIgnoreCase(".")) {
                    fileSave.setCurrentDirectory(new File(ConfigWindow.INPUT_DIRECTORY));
                } else {
                    fileSave.setCurrentDirectory(new File(ConfigWindow.OUTPUT_DIRECTORY));
                }

                fileSave.setSelectedFile(new File(file.getName().split("\\.")[0] + ".txt"));
                fileSave.setFileFilter(new FileNameExtensionFilter(".txt", "txt"));
                fileSave.setAcceptAllFileFilterUsed(false);
                fileSave.setPreferredSize(new Dimension(600, 600));
                fileSave.setDialogTitle("Export to TXT file.");
                int ret = fileSave.showDialog(null, "Export");
                if (ret == 0) {
                    this.progressTask = new ExportTxt(this, fileSave.getSelectedFile());
                    this.executorService.execute(this.progressTask);
                }
            } else {
                addErrorConsole("Cannot export to this file!", true);
            }

        }
    }

    private void saveActionPerformed(ActionEvent evt) {
        if (this.progressTask == null) {
            if (this.currentFileWindow != null) {
                this.progressTask = new SaveDat(this, this.currentFileWindow);
                this.executorService.execute(this.progressTask);
            } else {
                addErrorConsole("Error saving dat. No file name.", true);
            }

        }
    }

    private void abortActionPerformed(ActionEvent evt) {
        if (this.progressTask != null) {
            this.progressTask.abort();
            addLogConsole("---------------------------------------", true);
            addErrorConsole("Progress aborted.", true);
        }
    }

    public DatCrypter getEncryptor(File file) {
        DatCrypter crypter = null;
        String encryptorName = ConfigWindow.CURRENT_ENCRYPT;
        if (!encryptorName.equalsIgnoreCase(".") && !encryptorName.equalsIgnoreCase("Source") && !StringUtils.isEmpty(encryptorName.trim())) {
            crypter = CryptVersionParser.getInstance().getEncryptKey(encryptorName);
            if (crypter == null) {
                addErrorConsole("Not found " + encryptorName + " encryptor of the file: " + this.currentFileWindow.getName(), true);
            }
        } else {
            DatCrypter lastDatDecryptor = OpenDat.getLastDatCrypter(file);
            if (lastDatDecryptor != null) {
                crypter = CryptVersionParser.getInstance().getEncryptKey(lastDatDecryptor.getName());
                if (crypter == null) {
                    addErrorConsole("Not found " + lastDatDecryptor.getName() + " encryptor of the file: " + this.currentFileWindow.getName(), true);
                }
            }
        }

        return crypter;
    }

    private void saveComboBox(JComboBox<String> jComboBox, String param) {
        ConfigWindow.save(param, String.valueOf(jComboBox.getSelectedItem()));
    }

    public void onStartTask() {
        this.setCursor(Cursor.getPredefinedCursor(Cursor.WAIT_CURSOR));
        this.progressBar.setValue(0);
        this.checkButtons();
    }

    public void onProgressTask(int val) {
        this.progressBar.setValue(val);
    }

    public void onStopTask() {
        this.progressTask = null;
        this.progressBar.setValue(100);
        this.checkButtons();
        Toolkit.getDefaultToolkit().beep();
        this.setCursor(null);
    }

    public void onAbortTask() {
        if (this.progressTask != null) {
            this.progressTask = null;
            this.setCursor(null);
            this.checkButtons();
        }
    }

    private void checkButtons() {
        if (this.progressTask != null) {
            this.actionPanels.forEach((p) -> {
                for (Component c : p.getComponents()) {
                    c.setEnabled(false);
                }

            });
            this.abortTaskButton.setEnabled(true);
        } else {
            this.actionPanels.forEach((p) -> {
                for (Component c : p.getComponents()) {
                    if (c == this.exportTxtButton) {
                        c.setEnabled(this.currentFileWindow != null);
                    } else if (c == this.saveButton) {
                        c.setEnabled(this.currentFileWindow != null);
                    } else {
                        c.setEnabled(true);
                    }
                }

            });
            this.abortTaskButton.setEnabled(false);
        }

    }

    private static class LineNumberingTextArea extends JTextArea implements DocumentListener {
        private final JTextArea textArea;
        private int lastLines = 0;

        public LineNumberingTextArea(JTextArea textArea) {
            this.textArea = textArea;
        }

        public void cleanUp() {
            this.setText("");
            this.removeAll();
            this.lastLines = 0;
        }

        private void updateText() {
            int length = this.textArea.getLineCount();
            if (length != this.lastLines) {
                this.lastLines = length;
                StringBuilder lineNumbersTextBuilder = new StringBuilder();
                lineNumbersTextBuilder.append("1").append(System.lineSeparator());

                for (int line = 2; line <= length; ++line) {
                    lineNumbersTextBuilder.append(line).append(System.lineSeparator());
                }

                this.setText(lineNumbersTextBuilder.toString());
            }
        }

        public void insertUpdate(DocumentEvent documentEvent) {
            this.updateText();
        }

        public void removeUpdate(DocumentEvent documentEvent) {
            this.updateText();
        }

        public void changedUpdate(DocumentEvent documentEvent) {
            this.updateText();
        }
    }
}