package com.majestic.studio.xml;

public enum ParamNodeType {
    FOR,
    WRAPPER,
    CONSTANT,
    VARIABLE,
    IF,
    ELSE,
    MASK;

    boolean isCycle() {
        return this == FOR;
    }

    public boolean isWrapper() {
        return this == WRAPPER;
    }

    boolean isConstant() {
        return this == CONSTANT;
    }

    boolean isVariable() {
        return this == VARIABLE;
    }

    boolean isIf() {
        return this == IF;
    }

    boolean isElse() {
        return this == ELSE;
    }

    boolean isMask() {
        return this == MASK;
    }
}
