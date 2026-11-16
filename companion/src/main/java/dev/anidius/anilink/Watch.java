package dev.anidius.anilink;

import com.google.gson.JsonObject;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.ServerData;
import net.minecraft.client.server.IntegratedServer;
import net.minecraft.world.level.storage.LevelResource;

import java.io.File;
import java.nio.file.Path;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Objects;
import java.util.Set;

/** Runs once a second on the game thread: which world we are in, and new screenshots. */
final class Watch {
	private static String worldKey;
	private static JsonObject worldMsg;
	private static Set<String> knownShots;
	private static final Map<String, Long> pendingShots = new HashMap<>();

	private Watch() {}

	/** After a reconnect the launcher needs the current state again. */
	static void resend() {
		JsonObject m = worldMsg;
		if (m != null) Link.send(m);
	}

	static void poll() {
		try {
			pollWorld();
		} catch (Throwable t) {
			// never disturb the game
		}
		try {
			pollShots();
		} catch (Throwable t) {
			// same
		}
	}

	private static void pollWorld() {
		Minecraft mc = Minecraft.getInstance();
		JsonObject msg = new JsonObject();
		msg.addProperty("type", "world");
		String key = null;
		if (mc.level != null && mc.player != null) {
			IntegratedServer sp = mc.getSingleplayerServer();
			ServerData mp = mc.getCurrentServer();
			if (sp != null) {
				Path root = sp.getWorldPath(LevelResource.ROOT).toAbsolutePath().normalize();
				String folder = root.getFileName() == null ? "" : root.getFileName().toString();
				msg.addProperty("state", "enter");
				msg.addProperty("kind", "singleplayer");
				msg.addProperty("id", folder);
				msg.addProperty("name", sp.getWorldData().getLevelName());
				key = "sp:" + folder;
			} else if (mp != null) {
				msg.addProperty("state", "enter");
				msg.addProperty("kind", "multiplayer");
				msg.addProperty("id", mp.ip);
				msg.addProperty("name", mp.name);
				key = "mp:" + mp.ip;
			} else {
				msg.addProperty("state", "enter");
				msg.addProperty("kind", "other");
				msg.addProperty("id", "");
				msg.addProperty("name", "");
				key = "other";
			}
		} else {
			msg.addProperty("state", "leave");
		}
		if (!Objects.equals(key, worldKey)) {
			worldKey = key;
			worldMsg = msg;
			Link.send(msg);
		}
	}

	private static void pollShots() {
		Minecraft mc = Minecraft.getInstance();
		File dir = new File(mc.gameDirectory, "screenshots");
		String[] names = dir.list((d, n) -> n.toLowerCase().endsWith(".png"));
		if (names == null) names = new String[0];
		if (knownShots == null) {
			knownShots = new HashSet<>(java.util.Arrays.asList(names));
			return;
		}
		long now = System.currentTimeMillis();
		for (String n : names) {
			if (knownShots.contains(n)) continue;
			File f = new File(dir, n);
			// the game writes the file in the background; wait until it stops growing
			Long size = pendingShots.get(n);
			long cur = f.length();
			if (size == null || size != cur || cur == 0 || now - f.lastModified() < 400) {
				pendingShots.put(n, cur);
				continue;
			}
			pendingShots.remove(n);
			knownShots.add(n);
			JsonObject msg = new JsonObject();
			msg.addProperty("type", "screenshot");
			msg.addProperty("file", n);
			if (worldMsg != null && worldMsg.has("name")) {
				msg.addProperty("world", worldMsg.get("name").getAsString());
				msg.addProperty("kind", worldMsg.get("kind").getAsString());
			}
			if (mc.player != null && mc.level != null) {
				msg.addProperty("x", (int) Math.floor(mc.player.getX()));
				msg.addProperty("y", (int) Math.floor(mc.player.getY()));
				msg.addProperty("z", (int) Math.floor(mc.player.getZ()));
				msg.addProperty("dim", dimension(mc.level.dimension().toString()));
			}
			Link.send(msg);
		}
	}

	/** "ResourceKey[minecraft:dimension / minecraft:overworld]" → "minecraft:overworld" */
	private static String dimension(String key) {
		int i = key.lastIndexOf(" / ");
		String s = i >= 0 ? key.substring(i + 3) : key;
		return s.endsWith("]") ? s.substring(0, s.length() - 1) : s;
	}
}
