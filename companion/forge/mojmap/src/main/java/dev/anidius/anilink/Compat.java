package dev.anidius.anilink;

import net.minecraft.client.Minecraft;

import java.lang.reflect.Field;
import java.lang.reflect.Method;

/**
 * Forge 1.20.6+ and NeoForge 1.20.5+ run on Mojang's names, so one jar serves every version:
 * the calls that moved between versions are looked up by name.
 */
final class Compat {
	private Compat() {}

	static void toast(String title, String text) {
		Toasts.show(toastManager(), title, text);
	}

	private static Object toastManager() {
		Minecraft mc = Minecraft.getInstance();
		for (String name : new String[] {"getToastManager", "getToasts"}) {
			try {
				return Minecraft.class.getMethod(name).invoke(mc);
			} catch (Throwable ignored) {
				// moved in this version
			}
		}
		try {
			Field gui = Minecraft.class.getField("gui");
			Object g = gui.get(mc);
			return g.getClass().getMethod("toastManager").invoke(g);
		} catch (Throwable ignored) {
			return null;
		}
	}

	static void resizeGui() {
		Minecraft mc = Minecraft.getInstance();
		for (String name : new String[] {"resizeGui", "resizeDisplay"}) {
			try {
				Method m = Minecraft.class.getMethod(name);
				m.invoke(mc);
				return;
			} catch (Throwable ignored) {
				// try the other name
			}
		}
	}
}
