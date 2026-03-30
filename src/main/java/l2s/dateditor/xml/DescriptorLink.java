package l2s.dateditor.xml;

public class DescriptorLink {
    private final String namePattern;
    private final String linkFile;
    private final String linkVersion;

    public DescriptorLink(String namePattern, String linkFile, String linkVersion) {
        this.namePattern = namePattern;
        this.linkFile = linkFile;
        this.linkVersion = linkVersion;
    }

    public String getNamePattern() {
        return this.namePattern;
    }

    public String getLinkFile() {
        return this.linkFile;
    }

    public String getLinkVersion() {
        return this.linkVersion;
    }
}
