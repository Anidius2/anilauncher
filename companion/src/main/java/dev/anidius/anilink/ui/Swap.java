package dev.anidius.anilink.ui;

import net.minecraft.client.gui.screens.PauseScreen;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.TitleScreen;

/** Puts our menus in place of the game's title and pause screens (only these exact classes). */
public final class Swap {
	private Swap() {}

	public static Screen swap(Screen s) {
		if (s == null || Nav.bypass) return s;
		try {
			UiConfig cfg = UiConfig.get();
			if (cfg.menu && s.getClass() == TitleScreen.class) return new MenuScreen();
			if (cfg.pause && s.getClass() == PauseScreen.class && Nav.showsPauseMenu((PauseScreen) s)) return new PauseMenu();
		} catch (Throwable t) {
			// never let a menu stop the game: fall back to the game's own screen
			System.out.println("[AniLink] menu: " + t);
		}
		return s;
	}
}
