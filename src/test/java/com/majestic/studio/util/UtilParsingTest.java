package com.majestic.studio.util;

import org.junit.jupiter.api.Test;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Random;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import static org.junit.jupiter.api.Assertions.assertEquals;

class UtilParsingTest {
    private static final Pattern TEXT_PATTERN = Pattern.compile("\\[(.*?)]");
    private static final Pattern MAP_PATTERN = Pattern.compile("\\b(\\S+)\\b\\s*=\\s*(.*?)\t");

    @Test
    void preservesEscapingDuplicateFieldsAndFieldOrder() {
        String[] cases = {
                "id=1\tname=[plain]\tid=2\tempty=[]",
                "name=[a=b]\tother=[a\tb]\tcombined=[a=b\tc]\ttail=9",
                "first=[same=1]\tsecond=[same=1]\tthird=[outer[inner=2]]",
                "name=[ação 漢字]\ttext=[line\\r\\nnext]\tempty=\ttail=7",
                "name=[%_$tab$_%%_$eq$_%]\tbroken=[unterminated\tx=2",
                "name=[line\r\nnext]\tother=[\t=]\tlast=3",
                "", "\t", "prefix\tkey = [value]\tkey=[new]"
        };
        for (String input : cases) {
            assertSameFields(input);
        }
    }

    @Test
    void preservesLegacyInterpretationAcrossMixedBracketedValues() {
        Random random = new Random(20260907);
        String[] parts = {"plain", "=", "\t", "[", "]", " ", "漢", "ação", "\r", "\n", "%_$eq$_%", "%_$tab$_%", "\\r\\n"};
        for (int sample = 0; sample < 2000; sample++) {
            StringBuilder input = new StringBuilder();
            for (int field = 0; field < 8; field++) {
                input.append("key").append(random.nextInt(5)).append("=[");
                for (int part = 0; part < 5; part++) {
                    input.append(parts[random.nextInt(parts.length)]);
                }
                input.append("]\t");
            }
            assertSameFields(input.toString());
        }
    }

    private static void assertSameFields(String input) {
        Map<String, String> expected = legacyParse(input);
        Map<String, String> actual = Util.stringToMap(input);
        assertEquals(new ArrayList<>(expected.entrySet()), new ArrayList<>(actual.entrySet()), input);
    }

    // Compatibility oracle: the original parser's order of global replacements matters.
    private static Map<String, String> legacyParse(String input) {
        input += "\t";
        Map<String, String> result = new LinkedHashMap<>();
        String text;
        for (Matcher matcher = TEXT_PATTERN.matcher(input); matcher.find(); input = input.replace(text, text.replace("=", "%_$eq$_%"))) {
            text = matcher.group();
            input = input.replace(text, text.replace("\t", "%_$tab$_%"));
        }
        Matcher matcher = MAP_PATTERN.matcher(input);
        while (matcher.find()) {
            result.put(matcher.group(1), matcher.group(2).replace("%_$tab$_%", "\t").replace("%_$eq$_%", "="));
        }
        return result;
    }
}
