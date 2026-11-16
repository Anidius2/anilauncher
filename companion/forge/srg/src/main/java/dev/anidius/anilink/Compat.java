package dev.anidius.anilink;

import net.minecraft.client.Minecraft;

/** Forge 1.20–1.20.4 and NeoForge 1.20.2–1.20.4: obfuscated (SRG) names, stable across these versions. */
final class Compat {
	private Compat() {}

	static void toast(String title, String text) {
		Toasts.show(Minecraft.getInstance().getToasts(), title, text);
	}

	static void resizeGui() {
		Minecraft.getInstance().resizeDisplay();
	}
}
