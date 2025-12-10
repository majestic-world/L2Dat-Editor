import l2s.dateditor.actions.ActionTask;
import l2s.dateditor.listeners.FormatListener;
import l2s.dateditor.util.Util;
import org.apache.commons.lang3.StringUtils;

import java.util.Collections;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class SkillGrpFormat245 implements FormatListener {
	private static final Pattern pattern = Pattern.compile("\\bskill_autouse_begin\\b(.*?\\s)skill_autouse_end\\b", Pattern.DOTALL);
	private static final Pattern pattern2 = Pattern.compile("\\bskill_begin\\b(.*?\\s)skill_end\\b", Pattern.DOTALL);
	private static final Pattern pattern3 = Pattern.compile("\\bicon_panel_2_begin\\b(.*?\\s)icon_panel_2_end\\b", Pattern.DOTALL);

	private static int getSkillLevelMask(int skillLevel, int subSkillLevel) {
		return skillLevel | (subSkillLevel << 16);
	}

	private static int getSkillLevelFromMask(int skillLevelMask) {	
		final int mask = 0b1111111111111111;
		return mask & skillLevelMask;
	}

	private static int getSubSkillLevelFromMask(int skillLevelMask) {	
		final int mask = 0b1111111111111111;
		return mask & skillLevelMask >>> 16;
	}

	@Override
	public String decode(ActionTask actionTask, double progressWeight, String str) {
		int lineCount = str.split("\r\n|\r|\n").length;
		if (lineCount == 0) {
			return StringUtils.EMPTY;
		}

		double progress = actionTask.getCurrentProgress() ;
		double progressDiff = 100. / lineCount;

		Matcher m = pattern.matcher(str);
		Map<Integer, Integer> autoUses = new HashMap<>();
		while (m.find()) {
			Map<String, String> params = Util.stringToMap(m.group(1));
			int skillId = Integer.parseInt(params.get("skill_id"));
			int autoUseType = Integer.parseInt(params.get("auto_use_type"));
			autoUses.put(skillId, autoUseType);

			if (actionTask.isCancelled())
				return null;
			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}
		
		m = pattern3.matcher(str);
		Map<Integer, Map<Integer, String>> iconPanels2 = new HashMap<>();
		while (m.find()) {
			Map<String, String> params = Util.stringToMap(m.group(1));
			int skillId = Integer.parseInt(params.get("skill_id2"));
			int skillLvl = Integer.parseInt(params.get("skill_level2"));
			int skillSubLvl = Integer.parseInt(params.get("skill_sublevel2"));
			String iconPanel2 = params.get("icon_panel2");
			iconPanels2.computeIfAbsent(skillId, k -> new HashMap<>()).put(getSkillLevelMask(skillLvl, skillSubLvl), iconPanel2);

			if (actionTask.isCancelled())
				return null;
			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}

		StringBuilder builder = new StringBuilder();
		Matcher m2 = pattern2.matcher(str);
		while (m2.find()) {
			Map<String, String> params = Util.stringToMap(m2.group(1));
			int skillId = Integer.parseInt(params.get("id"));
			int skillLvl = Integer.parseInt(params.get("level"));
			int skillSubLvl = Integer.parseInt(params.get("sublevel"));
			Integer autoUse = autoUses.get(skillId);
			if(autoUse != null) {
				params.put("auto_use_type", String.valueOf(autoUse));
			} else {
				params.put("auto_use_type", "0");
			}
			String iconPanel2 = iconPanels2.getOrDefault(skillId, Collections.emptyMap()).getOrDefault(getSkillLevelMask(skillLvl, skillSubLvl), "[]");
			params.put("icon_panel_2", iconPanel2);
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

		double progress = actionTask.getCurrentProgress() ;
		double progressDiff = 100. / lineCount;

		StringBuilder builder = new StringBuilder();
		Map<Integer, Integer> autoUses = new LinkedHashMap<>();
		Map<Integer, Map<Integer, String>> iconPanels2 = new LinkedHashMap<>();
		Matcher m2 = pattern2.matcher(str);
		while (m2.find()) {
			Map<String, String> params = Util.stringToMap(m2.group(1));
			int id = Integer.parseInt(params.get("id"));
			int level = Integer.parseInt(params.get("level"));
			int sublevel = Integer.parseInt(params.get("sublevel"));
			String autoUseType = params.remove("auto_use_type");
			int type = autoUseType == null ? 0 : Integer.parseInt(autoUseType);
			if (type != 0)
				autoUses.put(id, type);
			String iconPanel2 = params.remove("icon_panel_2");
			if(iconPanel2 != null && !iconPanel2.equalsIgnoreCase("[]"))
				iconPanels2.computeIfAbsent(id, k -> new LinkedHashMap<>()).put(getSkillLevelMask(level, sublevel), iconPanel2);
			builder.append("skill_begin\t").append(Util.mapToString(params)).append("skill_end\r\n");

			if (actionTask.isCancelled())
				return null;
			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}

		for (Map.Entry<Integer, Integer> entry : autoUses.entrySet()) {
			builder.append("skill_autouse_begin\tskill_id=").append(entry.getKey()).append("\tauto_use_type=").append(entry.getValue()).append("\tskill_autouse_end\r\n");

			if (actionTask.isCancelled())
				return null;
			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}

		for (Map.Entry<Integer, Map<Integer, String>> entry : iconPanels2.entrySet()) {
			int skillId = entry.getKey();
			for (Map.Entry<Integer, String> entry1 : entry.getValue().entrySet()) {
				int skillLvl = getSkillLevelFromMask(entry1.getKey());
				int skillSubLvl = getSubSkillLevelFromMask(entry1.getKey());
				builder.append("icon_panel_2_begin\tskill_id2=").append(skillId)
					.append("\tskill_level2=").append(skillLvl)
					.append("\tskill_sublevel2=").append(skillSubLvl)
					.append("\ticon_panel2=").append(entry1.getValue()).append("\ticon_panel_2_end\r\n");
			}

			if (actionTask.isCancelled())
				return null;
			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}
		return builder.toString();
	}
}
