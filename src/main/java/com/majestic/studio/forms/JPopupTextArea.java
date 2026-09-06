package com.majestic.studio.forms;

import javax.swing.*;
import javax.swing.event.DocumentEvent;
import javax.swing.event.DocumentListener;
import javax.swing.undo.UndoManager;
import java.awt.*;
import java.awt.event.*;
import java.util.ArrayList;
import java.util.List;
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
        new SearchDialog().setVisible(true);
    }

    private class SearchDialog extends JDialog {
        private final JTextField query = new JTextField(28);
        private final JLabel results = new JLabel("0 results");
        private final JButton previous = new JButton("Previous");
        private final JButton next = new JButton("Next");
        private final List<Integer> matches = new ArrayList<>();
        private final String editorText = JPopupTextArea.this.getText();
        private int current = -1;

        private SearchDialog() {
            super(SwingUtilities.getWindowAncestor(JPopupTextArea.this),
                    "Search", Dialog.ModalityType.APPLICATION_MODAL);
            setDefaultCloseOperation(WindowConstants.DISPOSE_ON_CLOSE);

            JLabel prompt = new JLabel("Search string:");
            prompt.setLabelFor(query);
            JPanel input = new JPanel(new BorderLayout(0, 6));
            input.add(prompt, BorderLayout.NORTH);
            input.add(query, BorderLayout.CENTER);
            input.add(results, BorderLayout.SOUTH);

            JButton close = new JButton("Close");
            JPanel buttons = new JPanel(new FlowLayout(FlowLayout.RIGHT));
            buttons.add(previous);
            buttons.add(next);
            buttons.add(close);
            JPanel content = new JPanel(new BorderLayout(0, 10));
            content.setBorder(BorderFactory.createEmptyBorder(12, 12, 12, 12));
            content.add(input, BorderLayout.CENTER);
            content.add(buttons, BorderLayout.SOUTH);
            setContentPane(content);

            previous.setEnabled(false);
            next.setEnabled(false);
            previous.addActionListener(e -> navigate(false));
            next.addActionListener(e -> navigate(true));
            query.addActionListener(e -> navigate(true));
            close.addActionListener(e -> dispose());
            getRootPane().setDefaultButton(next);
            query.getDocument().addDocumentListener(new DocumentListener() {
                public void insertUpdate(DocumentEvent e) {
                    updateMatches();
                }

                public void removeUpdate(DocumentEvent e) {
                    updateMatches();
                }

                public void changedUpdate(DocumentEvent e) {
                    updateMatches();
                }
            });
            pack();
            setResizable(false);
            setLocationRelativeTo(getOwner());
        }

        private void updateMatches() {
            matches.clear();
            current = -1;
            String term = query.getText();
            if (!term.isEmpty()) {
                int start = editorText.indexOf(term);
                while (start >= 0) {
                    matches.add(start);
                    start = editorText.indexOf(term, start + term.length());
                }
            }
            results.setText(matches.size() + " results");
            previous.setEnabled(!matches.isEmpty());
            next.setEnabled(!matches.isEmpty());
        }

        private void navigate(boolean forward) {
            if (matches.isEmpty()) {
                return;
            }
            if (current >= 0) {
                current = Math.floorMod(current + (forward ? 1 : -1), matches.size());
            } else if (forward) {
                current = 0;
                int anchor = JPopupTextArea.this.getSelectionEnd();
                for (int i = 0; i < matches.size(); i++) {
                    if (matches.get(i) >= anchor) {
                        current = i;
                        break;
                    }
                }
            } else {
                current = matches.size() - 1;
                int anchor = JPopupTextArea.this.getSelectionStart();
                for (int i = matches.size() - 1; i >= 0; i--) {
                    if (matches.get(i) < anchor) {
                        current = i;
                        break;
                    }
                }
            }
            int start = matches.get(current);
            JPopupTextArea.this.select(start, start + query.getText().length());
            JPopupTextArea.this.getCaret().setSelectionVisible(true);
            results.setText((current + 1) + " of " + matches.size() + " results");
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
