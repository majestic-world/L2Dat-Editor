package l2s.dateditor.xml;

import l2s.dateditor.listeners.FormatListener;

import java.util.List;

public class Descriptor {
    private final String _alias;
    private final String _filePattern;
    private final List<ParamNode> _nodes;
    private boolean _isRawData;
    private boolean _isSafePackage;
    private FormatListener _format;

    Descriptor(String alias, String filePattern, List<ParamNode> nodes) {
        this._alias = alias;
        this._filePattern = filePattern;
        this._nodes = nodes;
        this._isRawData = false;
    }

    void setIsRawData(boolean value) {
        this._isRawData = value;
    }

    public void setIsSafePackage(boolean value) {
        this._isSafePackage = value;
    }

    boolean isRawData() {
        return this._isRawData;
    }

    public boolean isSafePackage() {
        return this._isSafePackage;
    }

    public String getAlias() {
        return this._alias;
    }

    public String getFilePattern() {
        return this._filePattern;
    }

    List<ParamNode> getNodes() {
        return this._nodes;
    }

    public FormatListener getFormat() {
        return this._format;
    }

    public void setFormat(FormatListener format) {
        this._format = format;
    }
}
