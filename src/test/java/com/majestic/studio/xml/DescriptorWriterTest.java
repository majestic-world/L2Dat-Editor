package com.majestic.studio.xml;

import com.majestic.studio.actions.ActionTask;
import org.junit.jupiter.api.Test;

import java.io.File;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;

class DescriptorWriterTest {
    @Test
    void preservesRecordOrderUnicodeLengthsAndSafePackageTrailer() throws Exception {
        ParamNode record = new ParamNode("row", ParamNodeType.FOR, null);
        record.setSize(2);
        record.addSubNodes(List.of(
                new ParamNode("id", ParamNodeType.VARIABLE, ParamType.INT),
                new ParamNode("name", ParamNodeType.VARIABLE, ParamType.UNICODE)));
        Descriptor descriptor = new Descriptor("test", "test.dat", List.of(record));
        descriptor.setIsSafePackage(true);
        ActionTask task = new ActionTask(null) {
            protected void action() { }
        };
        task.removePropertyChangeListener(task);
        String text = "row_begin\tid=1\tname=[A]\trow_end\r\nrow_begin\tid=-2\tname=[漢]\trow_end\r\n";

        byte[] actual = DescriptorWriter.parseData(task, 100, new File("test.dat"), null, descriptor, text, false);

        ByteBuffer expected = ByteBuffer.allocate(33).order(ByteOrder.LITTLE_ENDIAN);
        expected.putInt(1).putInt(2).putChar('A');
        expected.putInt(-2).putInt(2).putChar('漢');
        expected.put((byte) 12).put("SafePackage\0".getBytes(StandardCharsets.US_ASCII));
        assertArrayEquals(expected.array(), actual);
    }
}
