package dev.anidius.anilink.ui;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import net.minecraft.client.Minecraft;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;

/**
 * What the launcher told us about this build: its name and art, the player, the launcher's colours
 * and which screens to restyle. Read once from the file named in -Danilink.ui.
 */
public final class UiConfig {
	public static final UiConfig NONE = new UiConfig();
	private static UiConfig current;

	public boolean menu, pause, loading, licensed;
	public String name = "", subtitle = "", nick = "";
	public Path art, head;
	public int bg = 0xFF1B1B1B, panel = 0xFF2E2E2E, text = 0xFFF2F2F2, muted = 0xFFB8B8B8, accent = 0xFF3C8527;
	public String worldId = "", worldName = "", server = "";
	public long worldTime, serverTime;

	public static synchronized UiConfig get() {
		if (current == null) current = load();
		return current;
	}

	private static UiConfig load() {
		String file = System.getProperty("anilink.ui");
		if (file == null || file.isEmpty()) return NONE;
		try {
			JsonObject o = JsonParser.parseString(Files.readString(Path.of(file), StandardCharsets.UTF_8)).getAsJsonObject();
			UiConfig c = new UiConfig();
			c.menu = bool(o, "menu");
			c.pause = bool(o, "pause");
			c.loading = bool(o, "loading");
			c.name = str(o, "name");
			c.subtitle = str(o, "subtitle");
			c.nick = str(o, "nick");
			c.licensed = bool(o, "licensed");
			c.art = path(o, "art");
			c.head = path(o, "head");
			JsonObject col = o.has("colors") ? o.getAsJsonObject("colors") : new JsonObject();
			c.bg = color(col, "bg", c.bg);
			c.panel = color(col, "panel", c.panel);
			c.text = color(col, "text", c.text);
			c.muted = color(col, "muted", c.muted);
			c.accent = color(col, "accent", c.accent);
			if (o.has("world") && o.get("world").isJsonObject()) {
				JsonObject w = o.getAsJsonObject("world");
				c.worldId = str(w, "id");
				c.worldName = str(w, "name");
				c.worldTime = w.has("time") ? w.get("time").getAsLong() : 0;
			}
			if (o.has("server") && o.get("server").isJsonObject()) {
				JsonObject s = o.getAsJsonObject("server");
				c.server = str(s, "address");
				c.serverTime = s.has("time") ? s.get("time").getAsLong() : 0;
			}
			return c;
		} catch (Exception e) {
			System.out.println("[AniLink] ui config: " + e);
			return NONE;
		}
	}

	private static boolean bool(JsonObject o, String k) {
		return o.has(k) && o.get(k).isJsonPrimitive() && o.get(k).getAsBoolean();
	}

	private static String str(JsonObject o, String k) {
		return o.has(k) && o.get(k).isJsonPrimitive() ? o.get(k).getAsString() : "";
	}

	private static Path path(JsonObject o, String k) {
		String s = str(o, k);
		if (s.isEmpty()) return null;
		Path p = Path.of(s);
		return Files.isRegularFile(p) ? p : null;
	}

	/** "#rrggbb" -> opaque ARGB. */
	private static int color(JsonObject o, String k, int fallback) {
		String s = str(o, k).trim();
		if (s.startsWith("#") && s.length() == 7) {
			try {
				return 0xFF000000 | Integer.parseInt(s.substring(1), 16);
			} catch (NumberFormatException ignored) {
			}
		}
		return fallback;
	}

	/** The game's language decides our own words: Russian or English. */
	public static boolean ru() {
		try {
			return Minecraft.getInstance().options.languageCode.startsWith("ru");
		} catch (Throwable t) {
			return false;
		}
	}

	public static String tr(String ru, String en) {
		return ru() ? ru : en;
	}

	public static int alpha(int argb, float a) {
		int base = (argb >>> 24) & 0xFF;
		int na = Math.max(0, Math.min(255, Math.round(base * a)));
		return (na << 24) | (argb & 0xFFFFFF);
	}

	public static int mix(int a, int b, float t) {
		int r = Math.round(((a >> 16) & 0xFF) * (1 - t) + ((b >> 16) & 0xFF) * t);
		int g = Math.round(((a >> 8) & 0xFF) * (1 - t) + ((b >> 8) & 0xFF) * t);
		int bl = Math.round((a & 0xFF) * (1 - t) + (b & 0xFF) * t);
		return 0xFF000000 | (r << 16) | (g << 8) | bl;
	}
}
