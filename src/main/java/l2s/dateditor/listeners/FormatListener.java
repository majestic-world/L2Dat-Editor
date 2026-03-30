package l2s.dateditor.listeners;

import l2s.dateditor.actions.ActionTask;

public interface FormatListener {
    String decode(ActionTask var1, double var2, String var4);

    String encode(ActionTask var1, double var2, String var4);
}
