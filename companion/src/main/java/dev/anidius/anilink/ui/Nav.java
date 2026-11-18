package dev.anidius.anilink.ui;

import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.client.gui.screens.options.OptionsScreen;

/** Opening screens: the calls moved around between versions. */
public final class Nav {
	/** Set while we open the game's own title or pause screen on purpose. */
	public static boolean bypass;

	private Nav() {}

	public static void open(Screen s) {
		Minecraft mc = Minecraft.getInstance();
		//? if >=26.2 {
		/*mc.gui.setScreen(s);
		*///?} else {
		mc.setScreen(s);
		//?}
	}

	/** Opens a vanilla screen that our menus would normally replace. */
	public static void openClassic(Screen s) {
		bypass = true;
		try {
			open(s);
		} finally {
			bypass = false;
		}
	}

	public static void classicTitle() {
		openClassic(new TitleScreen());
	}

	public static Screen options(Screen parent, boolean inGame) {
		Minecraft mc = Minecraft.getInstance();
		//? if >=26.1 && <26.3 {
		/*return new OptionsScreen(parent, mc.options, inGame);
		*///?} else {
		return new OptionsScreen(parent, mc.options);
		//?}
	}

	/** The loader's own list of mods: Mod Menu on Fabric/Quilt, the built-in list on Forge and NeoForge. */
	private static final String[] MOD_LISTS = {
		"com.terraformersmc.modmenu.gui.ModsScreen",
		"net.neoforged.neoforge.client.gui.ModListScreen",
		"net.minecraftforge.client.gui.ModListScreen",
	};
	private static Class<?> modList;
	private static boolean looked;

	private static Class<?> modList() {
		if (!looked) {
			looked = true;
			for (String name : MOD_LISTS) {
				try {
					modList = Class.forName(name);
					break;
				} catch (Throwable ignored) {
				}
			}
		}
		return modList;
	}

	public static boolean hasModMenu() {
		return modList() != null;
	}

	public static void openMods(Screen parent) {
		try {
			open((Screen) modList().getConstructor(Screen.class).newInstance(parent));
		} catch (Throwable t) {
			System.out.println("[AniLink] mod list: " + t);
		}
	}
}
