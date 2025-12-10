import l2s.dateditor.actions.ActionTask;
import l2s.dateditor.listeners.FormatListener;
import l2s.dateditor.util.DebugUtil;
import l2s.dateditor.util.Util;
import org.apache.commons.lang3.StringUtils;

import java.util.HashMap;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class CollectionFormat implements FormatListener {
	private static final Pattern pattern1 = Pattern.compile("\\bcollection_info_begin\\b(.*?\\s)collection_info_end\\b", Pattern.DOTALL);
	private static final Pattern pattern2 = Pattern.compile("\\bunk_507_data_begin\\b(.*?\\s)unk_507_data_end\\b", Pattern.DOTALL);
	private static final Pattern pattern3 = Pattern.compile("\\bcollection_server_group_begin\\b(.*?\\s)collection_server_group_end\\b", Pattern.DOTALL);
	private static final Pattern pattern4 = Pattern.compile("\\bcollection_begin\\b(.*?\\s)collection_end\\b", Pattern.DOTALL);

	@Override
	public String decode(ActionTask actionTask, double progressWeight, String str) {
		int lineCount = str.split("\r\n|\r|\n").length;
		if (lineCount == 0) {
			return StringUtils.EMPTY;
		}

		double progress = actionTask.getCurrentProgress();
		double progressDiff = 100. / lineCount;

		Matcher m = pattern1.matcher(str);
		Map<Integer, Integer> serverGroups = new HashMap<>();
		while (m.find()) {
			Map<String, String> params = Util.stringToMap(m.group(1));
			String collection_ID = params.get("collection_ID");
			if (collection_ID == null) {
				DebugUtil.getLogger().error("'collection_ID' param is null in line[" + m.group(0) + "]");
				return null;
			}
			String server_group_id = params.get("server_group_id");
			if (server_group_id == null) {
				DebugUtil.getLogger().error("'server_group_id' param is null in line[" + m.group(0) + "]");
				return null;
			}
			serverGroups.put(Integer.valueOf(collection_ID), Integer.valueOf(server_group_id));
			if (actionTask.isCancelled())
				return null;
			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}

		StringBuilder builder = new StringBuilder();
		m = pattern2.matcher(str);
		while (m.find()) {
			builder.append(m.group()).append("\r\n");
		}
		m = pattern3.matcher(str);
		while (m.find()) {
			builder.append(m.group()).append("\r\n");
		}
		m = pattern4.matcher(str);
		while (m.find()) {
			Map<String, String> params = Util.stringToMap(m.group(1));
			int id = Integer.parseInt(params.get("collection_ID"));
			params.put("server_group_id", String.valueOf(serverGroups.getOrDefault(id, -1)));
			builder.append("collection_begin\t").append(Util.mapToString(params)).append("collection_end").append("\r\n");

			if (actionTask.isCancelled())
				return null;
			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}

		return builder.toString();
	}

	@Override
	public String encode(ActionTask actionTask, double progressWeight, String str) {
		int lineCount = str.split("\r\n|\r|\n").length;
		if (lineCount == 0) {
			return StringUtils.EMPTY;
		}

		double progress = actionTask.getCurrentProgress();
		double progressDiff = 100. / lineCount;

		StringBuilder builder1 = new StringBuilder();
		Matcher m = pattern2.matcher(str);
		while (m.find()) {
			builder1.append(m.group()).append("\r\n");
		}
		m = pattern3.matcher(str);
		while (m.find()) {
			builder1.append(m.group()).append("\r\n");
		}

		Map<Integer, Integer> serverGroups = new HashMap<>();
		m = pattern4.matcher(str);
		while (m.find()) {
			Map<String, String> params = Util.stringToMap(m.group(1));
			int id = Integer.parseInt(params.get("collection_ID"));
			String serverGroupId = params.remove("server_group_id");
			if (serverGroupId != null) {
				int autoUseType = Integer.parseInt(serverGroupId);
				if (autoUseType != -1)
					serverGroups.put(id, autoUseType);
			}
			builder1.append("collection_begin\t").append(Util.mapToString(params)).append("collection_end\r\n");

			if (actionTask.isCancelled())
				return null;
			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}

		StringBuilder builder = new StringBuilder();
		for (Map.Entry<Integer, Integer> entry : serverGroups.entrySet()) {
			builder.append("collection_info_begin\tcollection_ID=").append(entry.getKey()).append("\tserver_group_id=").append(entry.getValue()).append("\tcollection_info_end\r\n");

			if (actionTask.isCancelled())
				return null;
			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}
		builder.append(builder1);
		return builder.toString();
	}
}
