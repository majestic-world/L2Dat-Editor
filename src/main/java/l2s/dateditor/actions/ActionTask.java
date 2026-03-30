package l2s.dateditor.actions;

import l2s.dateditor.Boot;

import javax.swing.*;
import java.beans.PropertyChangeEvent;
import java.beans.PropertyChangeListener;
import java.util.concurrent.ExecutionException;

public abstract class ActionTask extends SwingWorker<Void, Void> implements PropertyChangeListener {
    protected final Boot boot;
    private double progress = 0.0F;

    public ActionTask(Boot boot) {
        this.boot = boot;
        this.addPropertyChangeListener(this);
    }

    public Boot getBoot() {
        return this.boot;
    }

    public Void doInBackground() {
        this.boot.onStartTask();
        this.setProgress(0);
        this.action();
        if (!this.isCancelled()) {
            this.setProgress(100);
        }

        return null;
    }

    protected abstract void action();

    public final void abort() {
        if (!this.isCancelled()) {
            this.cancel(true);
            this.boot.onAbortTask();
        }
    }

    public double addProgress(double progress, double value, double weight) {
        this.progress = this.getWeightValue(value, weight) + progress;
        this.changeProgress(this.progress);
        return this.progress;
    }

    public double addProgress(double value, double weight) {
        return this.addProgress(this.progress, value, weight);
    }

    public void changeProgress(double value) {
        int intValue = (int) Math.max(0.0F, Math.min(100.0F, value));
        if (intValue > this.getProgress()) {
            this.setProgress(intValue);
        }

    }

    public double getCurrentProgress() {
        return this.progress;
    }

    public double getWeightValue(double value, double weight) {
        return value / (double) 100.0F * weight;
    }

    public void done() {
        if (!this.isCancelled()) {
            try {
                this.get();
            } catch (InterruptedException | ExecutionException e) {
                e.printStackTrace();
            }
        }

        this.boot.onStopTask();
    }

    public void propertyChange(PropertyChangeEvent evt) {
        if ("progress".equals(evt.getPropertyName())) {
            this.boot.onProgressTask((Integer) evt.getNewValue());
        }

    }
}
