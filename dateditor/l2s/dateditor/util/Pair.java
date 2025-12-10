package l2s.dateditor.util;

public class Pair<E, T> {
    private final E first;
    private final T second;

    public Pair(E first, T second) {
        this.first = first;
        this.second = second;
    }

    public E getFirst() {
        return this.first;
    }

    public T getSecond() {
        return this.second;
    }
}
