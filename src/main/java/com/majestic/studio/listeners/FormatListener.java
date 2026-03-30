package com.majestic.studio.listeners;

import com.majestic.studio.actions.ActionTask;

public interface FormatListener {
    String decode(ActionTask var1, double var2, String var4);

    String encode(ActionTask var1, double var2, String var4);
}
