import l2s.dateditor.actions.ActionTask;
import l2s.dateditor.listeners.FormatListener;
import l2s.dateditor.util.Util;
import org.apache.commons.lang3.StringUtils;

import java.util.HashMap;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class SkillGrpFormat implements FormatListener {
	private static final Pattern pattern = Pattern.compile("\\bskill_autouse_begin\\b(.*?\\s)skill_autouse_end\\b", Pattern.DOTALL);
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
		Map<Integer, Integer> autoUses = new HashMap<>();
		while (m.find()) {
			Map<String, String> params = Util.stringToMap(m.group(1));
			int skillId = Integer.parseInt(params.get("skill_id"));
			int autoUse = Integer.parseInt(params.get("auto_use"));
			autoUses.put(skillId, autoUse);

			if (actionTask.isCancelled())
				return null;
			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}

		StringBuilder builder = new StringBuilder();
		Matcher m2 = pattern2.matcher(str);
		while (m2.find()) {
			Map<String, String> params = Util.stringToMap(m2.group(1));
			int skillId = Integer.parseInt(params.get("id"));
			params.put("auto_use", String.valueOf(autoUses.getOrDefault(skillId, 0)));
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

		StringBuilder builder = new StringBuilder();
		Map<Integer, Integer> autoUses = new HashMap<>();
		Matcher m2 = pattern2.matcher(str);
		while (m2.find()) {
			Map<String, String> params = Util.stringToMap(m2.group(1));
			int id = Integer.parseInt(params.get("id"));
			String autoUse = params.remove("auto_use");
			if (autoUse != null) {
				int autoUseType = Integer.parseInt(autoUse);
				if (autoUseType != 0)
					autoUses.put(id, autoUseType);
			}
			builder.append("skill_begin\t").append(Util.mapToString(params)).append("skill_end\r\n");

			if (actionTask.isCancelled())
				return null;
			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}

		for (Map.Entry<Integer, Integer> entry : autoUses.entrySet()) {
			builder.append("skill_autouse_begin\tskill_id=").append(entry.getKey()).append("\tauto_use=").append(entry.getValue()).append("\tskill_autouse_end\r\n");

			if (actionTask.isCancelled())
				return null;
			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}
		return builder.toString();
	}
}
