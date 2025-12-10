package l2s.dateditor.xml;

import java.util.ArrayList;
import java.util.List;

public class ParamNode {
    private final ParamNodeType _entityType;
    private final ParamType _type;
    private String _name;
    private int _size = -1;
    private boolean _hidden = false;
    private String _cycleName;
    private ArrayList<ParamNode> _sub;
    private boolean _isIterator;
    private boolean _skipWriteSize;
    private String _paramIf;
    private String _valIf;
    private String paramMask;
    private String enumName;
    private int valMask;
    private String defaultValue = null;
    private String oldName = null;
    private boolean key = false;

    ParamNode(String name, ParamNodeType entityType, ParamType type) {
        this._name = name;
        this._entityType = entityType;
        this._type = type;
    }

    public int getSize() {
        return this._size;
    }

    public void setSize(int size) {
        this._size = size;
    }

    void setIterator() {
        this._isIterator = true;
    }

    void setHidden() {
        this._hidden = true;
    }

    boolean isIterator() {
        return this._isIterator && this._size < 0;
    }

    boolean isNameHidden() {
        return this._hidden;
    }

    public String getName() {
        return this._name;
    }

    public void setName(String name) {
        this._name = name;
    }

    ParamNodeType getEntityType() {
        return this._entityType;
    }

    public ParamType getType() {
        return this._type;
    }

    ParamNode copy() {
        ParamNode node = new ParamNode(this.getName(), this.getEntityType(), this.getType());
        if (this.isNameHidden()) {
            node.setHidden();
        }

        if (this.isIterator()) {
            node.setIterator();
        }

        node.setSkipWriteSize(this.isSkipWriteSize());
        node.setCycleName(this.getCycleName());
        node.setEnumName(this.getEnumName());
        node.setDefaultValue(this.getDefaultValue());
        node.setOldName(this.getOldName());
        node.setKey(this.isKey());
        if (this.getSubNodes() != null) {
            List<ParamNode> list = new ArrayList<>();

            for (ParamNode n : this.getSubNodes()) {
                ParamNode copyN = n.copy();
                list.add(copyN);
            }

            node.addSubNodes(list);
        }

        return node;
    }

    void addSubNodes(List<ParamNode> n) {
        if (this._sub == null) {
            this._sub = new ArrayList<>();
        }

        this._sub.addAll(n);
    }

    List<ParamNode> getSubNodes() {
        return this._sub;
    }

    public String toString() {
        return this._name + "[" + this._entityType + "][" + this._cycleName + "][" + this._type + "]";
    }

    boolean isSkipWriteSize() {
        return this._skipWriteSize;
    }

    void setSkipWriteSize(boolean skipWrite) {
        this._skipWriteSize = skipWrite;
    }

    String getParamIf() {
        return this._paramIf;
    }

    void setParamIf(String paramIf) {
        this._paramIf = paramIf;
    }

    String getValIf() {
        return this._valIf;
    }

    void setValIf(String valIf) {
        this._valIf = valIf;
    }

    String getCycleName() {
        return this._cycleName;
    }

    void setCycleName(String cycleName) {
        this._cycleName = cycleName;
    }

    public String getParamMask() {
        return this.paramMask;
    }

    public void setParamMask(String paramMask) {
        this.paramMask = paramMask;
    }

    public int getValMask() {
        return this.valMask;
    }

    public void setValMask(int valMask) {
        this.valMask = valMask;
    }

    public boolean isEnum() {
        return this.enumName != null;
    }

    public String getEnumName() {
        return this.enumName;
    }

    public void setEnumName(String enumName) {
        this.enumName = enumName;
    }

    public String getDefaultValue() {
        return this.defaultValue;
    }

    public void setDefaultValue(String defaultValue) {
        this.defaultValue = defaultValue;
    }

    public String getOldName() {
        return this.oldName;
    }

    public void setOldName(String oldName) {
        this.oldName = oldName;
    }

    public boolean isKey() {
        return this.key;
    }

    public void setKey(boolean key) {
        this.key = key;
    }
}
