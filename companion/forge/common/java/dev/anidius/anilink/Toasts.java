package dev.anidius.anilink;

import net.minecraft.client.Minecraft;
import net.minecraft.network.chat.Component;

import java.lang.reflect.Method;
import java.lang.reflect.Modifier;

/**
 * Notifications that work across Minecraft versions without knowing method names:
 * SystemToast's static "add" methods are found by their parameter types. If that fails,
 * the message goes to chat instead.
 */
final class Toasts {
	private static Object id;
	private static Method add;

	private Toasts() {}

	static void show(Object manager, String title, String text) {
		Component t = Component.literal(title);
		Component d = text.isEmpty() ? null : Component.literal(text);
		try {
			if (add == null) {
				Class<?> toast = Class.forName("net.minecraft.client.gui.components.toasts.SystemToast");
				for (Method m : toast.getDeclaredMethods()) {
					Class<?>[] p = m.getParameterTypes();
					if (Modifier.isStatic(m.getModifiers()) && Modifier.isPublic(m.getModifiers()) && p.length == 4
							&& p[0].isInstance(manager) && p[2] == Component.class && p[3] == Component.class) {
						Class<?> idType = p[1];
						if (idType.isEnum()) {
							Object found = null;
							for (Object c : idType.getEnumConstants()) {
								if (c.toString().equals("PERIODIC_NOTIFICATION")) found = c;
							}
							if (found == null) continue;
							id = found;
						} else {
							id = idType.getConstructor(long.class).newInstance(6000L);
						}
						add = m;
						break;
					}
				}
			}
			add.invoke(null, manager, id, t, d);
		} catch (Throwable e) {
			chat(title, text);
		}
	}

	static void chat(String title, String text) {
		try {
			Minecraft.getInstance().gui.getChat().addMessage(Component.literal("[" + title + "] " + text));
		} catch (Throwable ignored) {
			Link.log(title + ": " + text);
		}
	}
}
