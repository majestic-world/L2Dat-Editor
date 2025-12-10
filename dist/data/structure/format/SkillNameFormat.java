import l2s.dateditor.actions.ActionTask;
import l2s.dateditor.listeners.FormatListener;
import l2s.dateditor.util.DebugUtil;
import l2s.dateditor.util.Util;
import org.apache.commons.lang3.StringUtils;

import java.util.*;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class SkillNameFormat implements FormatListener {
	private static class SkillData implements Comparable<SkillData> {
		public final int id;
		public final int level;
		public final int subLevel;
		public final String data;

		public SkillData(int id, int level, int subLevel, String data) {
			this.id = id;
			this.level = level;
			this.subLevel = subLevel;
			this.data = data;
		}

		@Override
		public int compareTo(SkillData o) {
			int res = Integer.compare(id, o.id);
			if (res == 0) {
				res = Integer.compare(level, o.level);
				if (res == 0) {
					res = Integer.compare(subLevel, o.subLevel);
				}
			}
			return res;
		}
	}

	private static final Pattern pattern = Pattern.compile("\\bskill_txt_begin\\b(.*?\\s)skill_txt_end\\b", Pattern.DOTALL);
	private static final Pattern pattern2 = Pattern.compile("\\bskill_begin\\b(.*?\\s)skill_end\\b", Pattern.DOTALL);

	@Override
	public String decode(ActionTask actionTask, double progressWeight, String str) {
		int lineCount = str.split("\r\n|\r|\n").length;
		if (lineCount == 0) {
			return StringUtils.EMPTY;
		}

		double progress = actionTask.getCurrentProgress();
		double progressDiff = 100. / lineCount;

		Matcher m = pattern.matcher(str);
		Map<Integer, String> indexes = new HashMap<>();
		while (m.find()) {
			Map<String, String> params = Util.stringToMap(m.group(1));
			String text = params.get("text");
			if (text == null) {
				DebugUtil.getLogger().error("'text' param is null in line[" + m.group(0) + "]");
				return null;
			}
			String index = params.get("index");
			if (index == null) {
				DebugUtil.getLogger().error("'index' param is null in line[" + m.group(0) + "]");
				return null;
			}
			indexes.put(Integer.valueOf(index), text.substring(1, text.length() - 1));
			if (actionTask.isCancelled())
				return null;
			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}

		StringBuilder builder = new StringBuilder();
		Matcher m2 = pattern2.matcher(str);
		while (m2.find()) {
			Map<String, String> params = Util.stringToMap(m2.group(1));
			setNameByIndex(indexes, params, "name");
			setNameByIndex(indexes, params, "desc");
			setNameByIndex(indexes, params, "desc_param");
			setNameByIndex(indexes, params, "enchant_name");
			setNameByIndex(indexes, params, "enchant_name_param");
			setNameByIndex(indexes, params, "enchant_desc");
			setNameByIndex(indexes, params, "enchant_desc_param");

			builder.append("skill_begin\t").append(Util.mapToString(params)).append("skill_end").append("\r\n");
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

		Map<String, String> indexes = new LinkedHashMap<>();
		List<SkillData> sorted = new ArrayList<>();

		Matcher m2 = pattern2.matcher(str);
		while (m2.find()) {
			String line = m2.group(0);
			Map<String, String> params = Util.stringToMap(m2.group(1));
			setIndexByName(indexes, params, "name", line);
			setIndexByName(indexes, params, "desc", line);
			setIndexByName(indexes, params, "desc_param", line);
			setIndexByName(indexes, params, "enchant_name", line);
			setIndexByName(indexes, params, "enchant_name_param", line);
			setIndexByName(indexes, params, "enchant_desc", line);
			setIndexByName(indexes, params, "enchant_desc_param", line);

			String result = "skill_begin\t" + Util.mapToString(params) + "skill_end\r\n";

			int id = Integer.parseInt(params.get("skill_id"));
			int level = Integer.parseInt(params.get("skill_level"));
			int subLevel = Integer.parseInt(params.get("skill_sublevel"));

			sorted.add(new SkillData(id, level, subLevel, result));

			if (actionTask.isCancelled())
				return null;

			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}

		StringBuilder builder = new StringBuilder();
		for (String key : indexes.keySet()) {
			builder.append("skill_txt_begin\ttext=").append(key).append("\tindex=").append(indexes.get(key)).append("\tskill_txt_end\r\n");

			if (actionTask.isCancelled())
				return null;

			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}

		Collections.sort(sorted);

		sorted.forEach((s) -> builder.append(s.data));

		return builder.toString();
	}

	private void setNameByIndex(Map<Integer, String> indexes, Map<String, String> params, String paramName) {
		params.put(paramName, "[" + indexes.get(Integer.parseInt(params.get(paramName))) + "]");
	}

	private void setIndexByName(Map<String, String> indexes, Map<String, String> params, String paramName, String line) {
		String name = params.get(paramName);
		if (name == null) {
			DebugUtil.getLogger().error("Not found text for param name[" + paramName + "] in line: " + line);
			return;
		}
		if (indexes.containsKey(name)) {
			params.put(paramName, indexes.get(name));
			return;
		}
		String index = String.valueOf(indexes.size());
		indexes.put(name, index);
		params.put(paramName, index);
	}
}
