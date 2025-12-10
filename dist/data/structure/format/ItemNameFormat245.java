import l2s.dateditor.Boot;
import l2s.dateditor.actions.ActionTask;
import l2s.dateditor.config.ConfigWindow;
import l2s.dateditor.listeners.FormatListener;
import l2s.dateditor.util.Util;
import org.apache.commons.lang3.StringUtils;

import java.util.HashMap;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class ItemNameFormat245 implements FormatListener {
	private static final Pattern pattern = Pattern.compile("\\bitem_autouse_begin\\b(.*?\\s)item_autouse_end\\b", Pattern.DOTALL);
	private static final Pattern pattern2 = Pattern.compile("\\bitem_name_begin\\b(.*?\\s)item_name_end\\b", Pattern.DOTALL);
	private static final Pattern pattern3 = Pattern.compile("\\bitem_enchant_begin\\b(.*?\\s)item_enchant_end\\b", Pattern.DOTALL);

	private static class ItemEnchant {
		final Object keepTypeSelection;
		final String keepTypeEnchant;

		public ItemEnchant(Object keepTypeSelection, String keepTypeEnchant) {
			this.keepTypeSelection = keepTypeSelection;
			this.keepTypeEnchant = keepTypeEnchant;
		}
	}

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
			int itemId = Integer.parseInt(params.get("item_id"));
			int autouseType = Integer.parseInt(params.get("autouse_type"));
			autoUses.put(itemId, autouseType);

			if (actionTask.isCancelled())
				return null;
			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}

		m = pattern3.matcher(str);
		Map<Integer, ItemEnchant> unks = new HashMap<>();
		while (m.find()) {
			Map<String, String> params = Util.stringToMap(m.group(1));
			int itemId = Integer.parseInt(params.get("item_ex_id"));
			Object keepTypeSelection = !ConfigWindow.CURRENT_ENUM.equalsIgnoreCase(Boot.DISABLED_STR) ? params.get("keep_type_selection") : Integer.parseInt(params.get("keep_type_selection"));
			String keepTypeEnchant = params.get("keep_type_enchant");
			unks.put(itemId, new ItemEnchant(keepTypeSelection, keepTypeEnchant));

			if (actionTask.isCancelled())
				return null;
			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}

		StringBuilder builder = new StringBuilder();
		Matcher m2 = pattern2.matcher(str);
		while (m2.find()) {
			Map<String, String> params = Util.stringToMap(m2.group(1));
			int id = Integer.parseInt(params.get("id"));
			params.put("autouse_type", String.valueOf(autoUses.getOrDefault(id, 0)));
			ItemEnchant itemEnchant = unks.getOrDefault(id, new ItemEnchant(!ConfigWindow.CURRENT_ENUM.equalsIgnoreCase(Boot.DISABLED_STR) ? "" : "0", ""));
			params.put("keep_type_selection", String.valueOf(itemEnchant.keepTypeSelection));
			params.put("keep_type_enchant", itemEnchant.keepTypeEnchant);
			builder.append("item_name_begin\t").append(Util.mapToString(params)).append("item_name_end").append("\r\n");

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
		Map<Integer, ItemEnchant> unks = new HashMap<>();
		Matcher m2 = pattern2.matcher(str);
		while (m2.find()) {
			Map<String, String> params = Util.stringToMap(m2.group(1));
			int id = Integer.parseInt(params.get("id"));
			String autoUse = params.remove("autouse_type");
			if (autoUse != null) {
				int autoUseType = Integer.parseInt(autoUse);
				if (autoUseType != 0)
					autoUses.put(id, autoUseType);
			}
			if (!ConfigWindow.CURRENT_ENUM.equalsIgnoreCase(Boot.DISABLED_STR)) {
				String keepTypeSelection = params.remove("keep_type_selection");
				String keepTypeEnchant = params.remove("keep_type_enchant");
				if (!StringUtils.isEmpty(keepTypeSelection) && !StringUtils.isEmpty(keepTypeEnchant))
					unks.put(id, new ItemEnchant(keepTypeSelection, keepTypeEnchant));
			} else {
				int keepTypeSelectionInt = 0;
				String keepTypeSelection = params.remove("keep_type_selection");
				if (keepTypeSelection != null) {
					keepTypeSelectionInt = Integer.parseInt(keepTypeSelection);
				}
				String keepTypeEnchant = params.remove("keep_type_enchant");
				if (keepTypeSelectionInt >= 0 && !StringUtils.isEmpty(keepTypeEnchant))
					unks.put(id, new ItemEnchant(keepTypeSelectionInt, keepTypeEnchant));
			}

			builder.append("item_name_begin\t").append(Util.mapToString(params)).append("item_name_end\r\n");

			if (actionTask.isCancelled())
				return null;
			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}

		for (Map.Entry<Integer, Integer> entry : autoUses.entrySet()) {
			builder.append("item_autouse_begin\titem_id=").append(entry.getKey()).append("\tautouse_type=").append(entry.getValue()).append("\titem_autouse_end\r\n");

			if (actionTask.isCancelled())
				return null;
			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}

		for (Map.Entry<Integer, ItemEnchant> entry : unks.entrySet()) {
			builder.append("item_enchant_begin\titem_ex_id=").append(entry.getKey()).append("\tkeep_type_selection=").append(entry.getValue().keepTypeSelection).append("\tkeep_type_enchant=").append(entry.getValue().keepTypeEnchant).append("\titem_enchant_end\r\n");

			if (actionTask.isCancelled())
				return null;
			progress = actionTask.addProgress(progress, progressDiff, progressWeight);
		}
		return builder.toString();
	}
}
