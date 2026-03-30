package com.majestic.studio.forms;

import javax.swing.*;
import javax.swing.undo.UndoManager;
import java.awt.*;
import java.awt.event.*;
import java.util.StringTokenizer;
import java.util.Vector;

public class JPopupTextArea extends JTextArea {
    static final String COPY = "Copy (Ctrl + C)";
    static final String CUT = "Cut (Ctrl + X)";
    static final String PASTE = "Paste (Ctrl + V)";
    static final String DELETE = "Delete";
    static final String SELECTALL = "Select all (Ctrl + A)";
    static final String LINE = "Go to (Ctrl + G)";
    static final String FIND = "Search (Ctrl + F)";
    final Vector<Integer> lineLength = new Vector<>();
    private final UndoManager man;

    public JPopupTextArea() {
        this.addPopupMenu();
        this.man = new UndoManager();
        this.getDocument().addUndoableEditListener(this.man);
        Action undo = new AbstractAction() {
            public void actionPerformed(ActionEvent e) {
                if (JPopupTextArea.this.man.canUndo()) {
                    JPopupTextArea.this.man.undo();
                }

            }
        };
        Action redo = new AbstractAction() {
            public void actionPerformed(ActionEvent e) {
                if (JPopupTextArea.this.man.canRedo()) {
                    JPopupTextArea.this.man.redo();
                }

            }
        };
        InputMap imap = this.getInputMap();
        imap.put(KeyStroke.getKeyStroke("ctrl Z"), "undo");
        imap.put(KeyStroke.getKeyStroke("ctrl Y"), "redo");
        ActionMap amap = this.getActionMap();
        amap.put("undo", undo);
        amap.put("redo", redo);
    }

    public void discardAllEdits() {
        this.man.discardAllEdits();
    }

    public void cleanUp() {
        this.setText("");
        this.removeAll();
        this.discardAllEdits();
    }

    private void addPopupMenu() {
        JPopupMenu menu = new JPopupMenu();
        JMenuItem copyItem = new JMenuItem();
        copyItem.setAction(this.getActionMap().get("copy-to-clipboard"));
        copyItem.setText("Copy (Ctrl + C)");
        JMenuItem cutItem = new JMenuItem();
        cutItem.setAction(this.getActionMap().get("cut-to-clipboard"));
        cutItem.setText("Cut (Ctrl + X)");
        JMenuItem pasteItem = new JMenuItem();
        pasteItem.setAction(this.getActionMap().get("paste-from-clipboard"));
        pasteItem.setText("Paste (Ctrl + V)");
        JMenuItem deleteItem = new JMenuItem();
        deleteItem.setAction(this.getActionMap().get("delete-previous"));
        deleteItem.setText("Delete");
        JMenuItem selectAllItem = new JMenuItem();
        selectAllItem.setAction(this.getActionMap().get("select-all"));
        selectAllItem.setText("Select all (Ctrl + A)");
        JMenuItem selectLine = new JMenuItem();
        selectLine.addActionListener((e) -> this.goToLine());
        selectLine.setText("Go to (Ctrl + G)");
        JMenuItem selectFind = new JMenuItem();
        selectFind.addActionListener((e) -> this.searchString());
        selectFind.setText("Search (Ctrl + F)");
        menu.add(copyItem);
        menu.add(cutItem);
        menu.add(pasteItem);
        menu.add(deleteItem);
        menu.add(new JSeparator());
        menu.add(selectAllItem);
        menu.add(selectLine);
        menu.add(selectFind);
        this.add(menu);
        this.addMouseListener(new PopupTriggerMouseListener(menu, this));
        this.addKeyListener(new KeyListen());
    }

    private void goToLine() {
        int no = 0;

        boolean fnd;
        String lineno;
        do {
            fnd = true;
            lineno = JOptionPane.showInputDialog("Line number:");

            try {
                no = Integer.parseInt(lineno);
            } catch (Exception var6) {
                if (lineno != null) {
                    JOptionPane.showMessageDialog(new Frame(), "Enter a valid line number", "Error", 0);
                    fnd = false;
                }
            }

            if (no <= 0 && lineno != null) {
                JOptionPane.showMessageDialog(new Frame(), "Enter a valid line number", "Error", 0);
                fnd = false;
            }
        } while (!fnd);

        if (lineno != null) {
            this.getLinePosition();
            if (no - 1 >= this.lineLength.size()) {
                JOptionPane.showMessageDialog(new Frame(), "Line number does not exist", "Error", 0);
            } else {
                try {
                    this.requestFocus();
                    this.setCaretPosition(this.lineLength.elementAt(no - 1));
                } catch (Exception var5) {
                    JOptionPane.showMessageDialog(new Frame(), "Bad position", "Error", 0);
                }
            }
        }

    }

    private void searchString() {
        String lineno = JOptionPane.showInputDialog("Search string: ");
        if (lineno != null && !lineno.isEmpty()) {
            try {
                this.requestFocus();
                String editorText = this.getText();
                int start = editorText.indexOf(lineno, this.getSelectionEnd());
                if (start != -1) {
                    this.setCaretPosition(start);
                    this.moveCaretPosition(start + lineno.length());
                    this.getCaret().setSelectionVisible(true);
                }
            } catch (Exception var4) {
                JOptionPane.showMessageDialog(new Frame(), "Bad position", "Error", 0);
            }

        } else {
            JOptionPane.showMessageDialog(new Frame(), "Enter a empty string", "Error", 0);
        }
    }

    private void getLinePosition() {
        this.lineLength.clear();
        String txt = this.getText();
        int width = this.getWidth();
        StringTokenizer st = new StringTokenizer(txt, "\n ", true);
        String str = " ";
        int len = 0;
        this.lineLength.addElement(0);

        while (st.hasMoreTokens()) {
            String token = st.nextToken();
            int w = this.getGraphics().getFontMetrics(this.getGraphics().getFont()).stringWidth(str + token);
            if (w <= width && token.charAt(0) != '\n') {
                str = str + token;
            } else {
                len += str.length();
                if (token.charAt(0) == '\n') {
                    this.lineLength.addElement(len);
                } else {
                    this.lineLength.addElement(len - 1);
                }

                str = token;
            }
        }

    }

    private static class PopupTriggerMouseListener extends MouseAdapter {
        private final JPopupMenu popup;
        private final JComponent component;

        public PopupTriggerMouseListener(JPopupMenu popup, JComponent component) {
            this.popup = popup;
            this.component = component;
        }

        private void showMenuIfPopupTrigger(MouseEvent e) {
            if (e.isPopupTrigger()) {
                this.popup.show(this.component, e.getX() + 3, e.getY() + 3);
            }

        }

        public void mousePressed(MouseEvent e) {
            this.showMenuIfPopupTrigger(e);
        }

        public void mouseReleased(MouseEvent e) {
            this.showMenuIfPopupTrigger(e);
        }
    }

    private class KeyListen implements KeyListener {
        private boolean controlDown;
        private boolean gDown;
        private boolean fDown;

        private KeyListen() {
        }

        public void keyTyped(KeyEvent e) {
        }

        public void keyPressed(KeyEvent e) {
            if (e.getKeyCode() == 17) {
                this.controlDown = true;
            } else if (e.getKeyCode() == 71) {
                this.gDown = true;
            } else if (e.getKeyCode() == 70) {
                this.fDown = true;
            }

            if (this.controlDown) {
                if (this.gDown) {
                    this.controlDown = false;
                    this.gDown = false;
                    JPopupTextArea.this.goToLine();
                } else if (this.fDown) {
                    this.controlDown = false;
                    this.fDown = false;
                    JPopupTextArea.this.searchString();
                }
            }

        }

        public void keyReleased(KeyEvent e) {
            if (e.getKeyCode() == 17) {
                this.controlDown = false;
            } else if (e.getKeyCode() == 71) {
                this.gDown = false;
            } else if (e.getKeyCode() == 70) {
                this.fDown = false;
            }

        }
    }
}
