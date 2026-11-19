package dev.anidius.anilink.ui;

import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.TitleScreen;
//? if >=1.21 {
import net.minecraft.client.gui.screens.options.OptionsScreen;
//?} else {
/*import net.minecraft.client.gui.screens.OptionsScreen;
*///?}

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

	/** Leaves the world before 1.21.6 (newer versions have one call for the whole "Save and Quit"). */
	public static void dropWorld(boolean local) {
		Minecraft mc = Minecraft.getInstance();
		net.minecraft.network.chat.Component saving = net.minecraft.network.chat.Component.translatable("menu.savingLevel");
		//? if >=1.21.6 {
		return;
		//?} else if >=1.20.5 {
		/*if (local) mc.disconnect(new net.minecraft.client.gui.screens.GenericMessageScreen(saving));
		else mc.disconnect();
		*///?} else if >=1.20.2 {
		/*if (local) mc.disconnect(new net.minecraft.client.gui.screens.GenericDirtMessageScreen(saving));
		else mc.disconnect();
		*///?} else {
		/*if (local) mc.clearLevel(new net.minecraft.client.gui.screens.GenericDirtMessageScreen(saving));
		else mc.clearLevel();
		*///?}
	}

	public static Screen advancements(Minecraft mc, Screen parent) {
		//? if >=1.20.5 {
		return new net.minecraft.client.gui.screens.advancements.AdvancementsScreen(mc.player.connection.getAdvancements(), parent);
		//?} else {
		/*return new net.minecraft.client.gui.screens.advancements.AdvancementsScreen(mc.player.connection.getAdvancements());
		*///?}
	}

	/** Call before leaving: was this a Realms server? */
	public static boolean wasRealm(Minecraft mc, net.minecraft.client.multiplayer.ServerData server) {
		//? if >=1.20.2 {
		return server != null && server.isRealm();
		//?} else {
		/*return mc.isConnectedToRealms();
		*///?}
	}

	public static boolean showsPauseMenu(net.minecraft.client.gui.screens.PauseScreen s) {
		//? if >=1.20.2 {
		return s.showsPauseMenu();
		//?} else {
		/*return ((dev.anidius.anilink.mixin.PauseScreenAccess) s).anilink$showsMenu();
		*///?}
	}

	/** "Continue" for a singleplayer world. */
	public static void openWorld(Screen back, String world) {
		Minecraft mc = Minecraft.getInstance();
		//? if >=1.20.5 {
		mc.createWorldOpenFlows().openWorld(world, () -> open(back));
		//?} else if >=1.20.3 {
		/*mc.createWorldOpenFlows().checkForBackupAndLoad(world, () -> open(back));
		*///?} else {
		/*mc.createWorldOpenFlows().loadLevel(back, world);
		*///?}
	}

	/** "Continue" for a server. */
	public static void connect(Screen back, String address) {
		Minecraft mc = Minecraft.getInstance();
		var addr = net.minecraft.client.multiplayer.resolver.ServerAddress.parseString(address);
		//? if >=1.20.2 {
		var data = new net.minecraft.client.multiplayer.ServerData(address, address, net.minecraft.client.multiplayer.ServerData.Type.OTHER);
		//?} else {
		/*var data = new net.minecraft.client.multiplayer.ServerData(address, address, false);
		*///?}
		//? if >=1.20.5 {
		net.minecraft.client.gui.screens.ConnectScreen.startConnecting(back, mc, addr, data, false, null);
		//?} else {
		/*net.minecraft.client.gui.screens.ConnectScreen.startConnecting(back, mc, addr, data, false);
		*///?}
	}
}
