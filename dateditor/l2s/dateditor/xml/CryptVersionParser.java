package l2s.dateditor.xml;

import l2s.dateditor.clientcryptor.crypt.*;
import l2s.dateditor.util.DebugUtil;
import org.w3c.dom.Document;
import org.w3c.dom.Node;

import javax.xml.parsers.DocumentBuilderFactory;
import java.io.File;
import java.util.LinkedHashMap;
import java.util.Map;

public class CryptVersionParser {
    private static final CryptVersionParser INSTANCE = new CryptVersionParser();
    private final Map<String, DatCrypter> encryptKeys = new LinkedHashMap<>();
    private final Map<String, DatCrypter> decryptKeys = new LinkedHashMap<>();

    public static CryptVersionParser getInstance() {
        return INSTANCE;
    }

    public void parse() {
        File def = new File("./data/config/cryptVersion_Full.xml");
        if (!def.exists()) {
            def = new File("./data/config/cryptVersion.xml");
        }

        if (def.exists()) {
            DocumentBuilderFactory factory = DocumentBuilderFactory.newInstance();
            factory.setValidating(false);
            factory.setIgnoringElementContentWhitespace(true);
            factory.setIgnoringComments(true);

            try {
                Document doc = factory.newDocumentBuilder().parse(def);

                for (Node defsNode = doc.getFirstChild(); defsNode != null; defsNode = doc.getNextSibling()) {
                    if (defsNode.getNodeName().equals("keys")) {
                        for (Node defNode = defsNode.getFirstChild(); defNode != null; defNode = defNode.getNextSibling()) {
                            if (defNode.getNodeName().equals("key")) {
                                String name = defNode.getAttributes().getNamedItem("name").getNodeValue();
                                String type = defNode.getAttributes().getNamedItem("type").getNodeValue().toLowerCase();
                                int code = Integer.parseInt(defNode.getAttributes().getNamedItem("code").getNodeValue().toLowerCase());
                                boolean isDecrypt = Boolean.parseBoolean(defNode.getAttributes().getNamedItem("decrypt").getNodeValue());
                                boolean useStructure = Boolean.parseBoolean(defNode.getAttributes().getNamedItem("useStructure").getNodeValue());
                                String extension = defNode.getAttributes().getNamedItem("extension").getNodeValue();
                                DatCrypter dat = null;
                                switch (type) {
                                    case "rsa":
                                        String modulus = defNode.getAttributes().getNamedItem("modulus").getNodeValue();
                                        String exp = defNode.getAttributes().getNamedItem("exp").getNodeValue();
                                        dat = new RSADatCrypter(name, code, modulus, exp, isDecrypt);
                                        break;
                                    case "xor":
                                        dat = new XorDatCrypter(name, code, Integer.parseInt(defNode.getAttributes().getNamedItem("key").getNodeValue()), isDecrypt);
                                        break;
                                    case "blowfish":
                                        dat = new BlowFishDatCrypter(name, code, defNode.getAttributes().getNamedItem("key").getNodeValue(), isDecrypt);
                                        break;
                                    case "des":
                                        dat = new DESDatCrypter(name, code, defNode.getAttributes().getNamedItem("key").getNodeValue(), isDecrypt);
                                }

                                if (dat != null) {
                                    dat.addFileExtension(extension);
                                    dat.setUseStructure(useStructure);
                                    if (isDecrypt) {
                                        this.decryptKeys.put(name, dat);
                                    } else {
                                        this.encryptKeys.put(name, dat);
                                    }
                                }
                            }
                        }
                    }
                }
            } catch (Exception e) {
                DebugUtil.getLogger().error(e.getMessage(), e);
            }
        }

    }

    public Map<String, DatCrypter> getEncryptKey() {
        return this.encryptKeys;
    }

    public Map<String, DatCrypter> getDecryptKeys() {
        return this.decryptKeys;
    }

    public DatCrypter getEncryptKey(String s) {
        return this.encryptKeys.get(s);
    }

    public DatCrypter getDecryptKey(String s) {
        return this.decryptKeys.get(s);
    }
}
