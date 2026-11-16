package dev.anidius.anilink;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.mojang.blaze3d.platform.InputConstants;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Options;
import net.minecraft.server.packs.repository.PackRepository;
import net.minecraft.sounds.SoundSource;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;

/** Commands from the launcher. Always called on the game thread. */
final class Actions {
	private Actions() {}

	static void handle(JsonObject msg) {
		String cmd = str(msg, "cmd");
		JsonObject reply = new JsonObject();
		reply.addProperty("type", "reply");
		if (msg.has("id")) reply.add("id", msg.get("id"));
		try {
			switch (cmd) {
				case "toast" -> toast(str(msg, "title"), str(msg, "text"));
				case "listPacks" -> reply.add("packs", packs());
				case "enablePack" -> setPack(str(msg, "pack"), true);
				case "disablePack" -> setPack(str(msg, "pack"), false);
				case "getOptions" -> reply.add("options", options());
				case "setOptions" -> setOptions(msg.getAsJsonObject("values"));
				case "setKeys" -> setKeys(msg.getAsJsonObject("keys"));
				default -> throw new IllegalArgumentException("unknown command " + cmd);
			}
			reply.addProperty("ok", true);
		} catch (Throwable t) {
			reply.addProperty("ok", false);
			reply.addProperty("error", String.valueOf(t.getMessage()));
		}
		Link.send(reply);
	}

	private static String str(JsonObject o, String key) {
		JsonElement e = o.get(key);
		return e == null || e.isJsonNull() ? "" : e.getAsString();
	}

	// ---------- notifications ----------

	static void toast(String title, String text) {
		Compat.toast(title.isEmpty() ? "AniLauncher" : title, text);
	}

	// ---------- resource packs ----------

	private static JsonObject packs() {
		PackRepository repo = Minecraft.getInstance().getResourcePackRepository();
		repo.reload();
		JsonObject o = new JsonObject();
		JsonArray all = new JsonArray();
		for (String id : repo.getAvailableIds()) all.add(id);
		JsonArray on = new JsonArray();
		for (String id : repo.getSelectedIds()) on.add(id);
		o.add("available", all);
		o.add("selected", on);
		return o;
	}

	/** Turns a pack on (on top of the others) or off, then reloads resources like the packs screen does. */
	private static void setPack(String id, boolean on) {
		Minecraft mc = Minecraft.getInstance();
		PackRepository repo = mc.getResourcePackRepository();
		repo.reload();
		if (on && !repo.getAvailableIds().contains(id)) throw new IllegalArgumentException("pack not found: " + id);
		List<String> selected = new ArrayList<>(repo.getSelectedIds());
		selected.remove(id);
		if (on) selected.add(id);
		repo.setSelected(selected);
		mc.options.updateResourcePacks(repo);
	}

	// ---------- options ----------

	private static JsonObject options() {
		Options o = Minecraft.getInstance().options;
		JsonObject v = new JsonObject();
		v.addProperty("fov", o.fov().get());
		v.addProperty("renderDistance", o.renderDistance().get());
		v.addProperty("simulationDistance", o.simulationDistance().get());
		v.addProperty("guiScale", o.guiScale().get());
		v.addProperty("gamma", o.gamma().get());
		v.addProperty("maxFps", o.framerateLimit().get());
		v.addProperty("vsync", o.enableVsync().get());
		v.addProperty("master", o.getSoundSourceOptionInstance(SoundSource.MASTER).get());
		v.addProperty("music", o.getSoundSourceOptionInstance(SoundSource.MUSIC).get());
		return v;
	}

	private static void setOptions(JsonObject values) {
		Minecraft mc = Minecraft.getInstance();
		Options o = mc.options;
		boolean resize = false;
		for (Map.Entry<String, JsonElement> e : values.entrySet()) {
			JsonElement v = e.getValue();
			switch (e.getKey()) {
				case "fov" -> o.fov().set(clamp(v.getAsInt(), 30, 110));
				case "renderDistance" -> o.renderDistance().set(clamp(v.getAsInt(), 2, 32));
				case "simulationDistance" -> o.simulationDistance().set(clamp(v.getAsInt(), 5, 32));
				case "guiScale" -> {
					o.guiScale().set(clamp(v.getAsInt(), 0, 6));
					resize = true;
				}
				case "gamma" -> o.gamma().set(Math.max(0.0, Math.min(1.0, v.getAsDouble())));
				case "maxFps" -> o.framerateLimit().set(clamp(v.getAsInt(), 10, 260));
				case "vsync" -> o.enableVsync().set(v.getAsBoolean());
				case "master" -> o.getSoundSourceOptionInstance(SoundSource.MASTER).set(Math.max(0.0, Math.min(1.0, v.getAsDouble())));
				case "music" -> o.getSoundSourceOptionInstance(SoundSource.MUSIC).set(Math.max(0.0, Math.min(1.0, v.getAsDouble())));
				default -> { }
			}
		}
		if (resize) Compat.resizeGui();
		o.save();
	}

	private static int clamp(int v, int lo, int hi) {
		return Math.max(lo, Math.min(hi, v));
	}

	/** Same names as options.txt: "key_key.jump" → "key.keyboard.space". */
	private static void setKeys(JsonObject keys) {
		Options o = Minecraft.getInstance().options;
		for (KeyMapping km : o.keyMappings) {
			JsonElement v = keys.get("key_" + km.getName());
			if (v == null) continue;
			km.setKey(InputConstants.getKey(v.getAsString()));
		}
		KeyMapping.resetMapping();
		o.save();
	}
}
