package com.majestic.studio.xml;

import java.util.HashMap;
import java.util.Map;

public class DescriptorLinks {
    private final int id;
    private final int parentId;
    private final String name;
    private final Map<String, DescriptorLink> links = new HashMap<>();
    private DescriptorLinks parentLinks = null;

    public DescriptorLinks(int id, int parentId, String name) {
        this.id = id;
        this.parentId = parentId;
        this.name = name;
    }

    public int getId() {
        return this.id;
    }

    public int getParentId() {
        return this.parentId;
    }

    public String getName() {
        return this.name;
    }

    public void addLink(DescriptorLink link) {
        this.links.put(link.getNamePattern(), link);
    }

    public void setParentLinks(DescriptorLinks parentLinks) {
        this.parentLinks = parentLinks;
    }

    public Descriptor findDescriptor(String fileName, Map<String, Map<String, Descriptor>> descriptors) {
        for (DescriptorLink link : this.links.values()) {
            if (fileName.toLowerCase().matches(link.getNamePattern().toLowerCase()) && descriptors.containsKey(link.getLinkFile())) {
                Map<String, Descriptor> versions = descriptors.get(link.getLinkFile());
                if (versions.containsKey(link.getLinkVersion())) {
                    return versions.get(link.getLinkVersion());
                }
            }
        }

        if (this.parentLinks != null) {
            return this.parentLinks.findDescriptor(fileName, descriptors);
        } else {
            return null;
        }
    }
}
