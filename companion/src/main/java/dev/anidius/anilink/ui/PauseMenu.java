package dev.anidius.anilink.ui;

import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.screens.PauseScreen;
import net.minecraft.client.gui.screens.achievement.StatsScreen;
import net.minecraft.client.multiplayer.ServerData;
import net.minecraft.network.chat.Component;
import net.minecraft.client.gui.components.AbstractWidget;
import java.util.ArrayList;
import java.util.List;
//? if >=1.21.6 {
import net.minecraft.client.multiplayer.ClientLevel;
//?} else {
/*import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.client.gui.screens.multiplayer.JoinMultiplayerScreen;
import com.mojang.realmsclient.RealmsMainScreen;
*///?}

/** The pause menu as a side panel over the blurred game: where you are, how long you've played, the usual buttons. */
public final class PauseMenu extends BaseScreen {
	private static final int COL_W = 170;
	/** When the game process started: "playing" counts the whole session. */
	private static final long STARTED = java.lang.management.ManagementFactory.getRuntimeMXBean().getStartTime();
	private int bottom;

	public PauseMenu() {
		super(Component.translatable("menu.game"));
	}

	@Override
	protected void init() {
		Minecraft mc = Minecraft.getInstance();
		boolean ru = UiConfig.ru();
		int x = 16, gap = 4, h = 20;
		int y = 74;
		int half = (COL_W - gap) / 2;
		button(Component.translatable("menu.returnToGame"), x, y, COL_W, h, () -> Nav.open(null));
		y += h + gap + 4;
		if (mc.player != null) {
			button(Component.translatable("gui.advancements"), x, y, half, h,
				() -> Nav.open(Nav.advancements(mc, this)));
			button(Component.translatable("gui.stats"), x + half + gap, y, COL_W - half - gap, h,
				() -> Nav.open(new StatsScreen(this, mc.player.getStats())));
			y += h + gap;
		}
		if (Nav.hasModMenu()) {
			button(Component.translatable("menu.options"), x, y, half, h, () -> Nav.open(Nav.options(this, true)));
			button(Component.literal(ru ? "Моды" : "Mods"), x + half + gap, y, COL_W - half - gap, h, () -> Nav.openMods(this));
		} else {
			button(Component.translatable("menu.options"), x, y, COL_W, h, () -> Nav.open(Nav.options(this, true)));
		}
		y += h + gap;
		if (mc.hasSingleplayerServer() && mc.getSingleplayerServer() != null && !mc.getSingleplayerServer().isPublished()) {
			button(Component.literal(ru ? "Открыть для друзей" : "Open to friends"), x, y, COL_W, h, () -> openLan(mc));
			y += h + gap;
		}
		// buttons other mods put into the game's pause menu. Wide ones: one per row, or two per row when
		// there are several. Icons: beside "Classic menu" (up to three), the rest in rows under it.
		List<AbstractWidget> wide = new ArrayList<>(), icons = new ArrayList<>();
		// the game's own buttons (and Mod Menu's, which this menu already has) stay out
		foreignButtons(new PauseScreen(true), new String[] {"menu.", "gui.", "modmenu", "fml.menu"}, wide, icons);
		int pairW = (COL_W - gap) / 2;
		for (int i = 0; i < wide.size(); i++) {
			boolean pairs = wide.size() > 1;
			boolean right = pairs && i % 2 == 1;
			boolean lastAlone = pairs && i == wide.size() - 1 && i % 2 == 0;
			int w = !pairs || lastAlone ? COL_W : pairW;
			place(wide.get(i), right ? x + pairW + gap : x, y, w);
			if (!pairs || right || lastAlone) y += h + gap;
		}
		int beside = 0, besideW = 0;
		while (beside < icons.size() && beside < 3) besideW += icons.get(beside++).getWidth() + gap;
		button(Component.literal(ru ? "Обычное меню" : "Classic menu"), x, y, COL_W - besideW, h, () -> Nav.openClassic(new PauseScreen(true)));
		int ix = x + COL_W - besideW + gap;
		for (int i = 0; i < beside; i++) {
			AbstractWidget b = icons.get(i);
			place(b, ix, y + (h - b.getHeight()) / 2, b.getWidth());
			ix += b.getWidth() + gap;
		}
		y += h + gap;
		ix = x;
		for (int i = beside; i < icons.size(); i++) {
			AbstractWidget b = icons.get(i);
			if (ix + b.getWidth() > x + COL_W) {
				ix = x;
				y += h + gap;
			}
			place(b, ix, y + (h - b.getHeight()) / 2, b.getWidth());
			ix += b.getWidth() + gap;
		}
		if (beside < icons.size()) y += h + gap;
		y += 4;
		Component leave = mc.isLocalServer() ? Component.translatable("menu.returnToMenu") : Component.translatable("menu.disconnect");
		button(leave, x, y, COL_W, h, () -> leave(mc));
		bottom = y + h;
	}

	private void openLan(Minecraft mc) {
		//? if >=26.3 {
		/*Nav.open(new net.minecraft.client.gui.screens.WorldOptionsScreen(this, mc.level));
		*///?} else if >=26.2 {
		/*Nav.open(new net.minecraft.client.gui.screens.options.WorldOptionsScreen(this, mc.level));
		*///?} else {
		Nav.open(new net.minecraft.client.gui.screens.ShareToLanScreen(this));
		//?}
	}

	/** Same as the game's own "Save and Quit" / "Disconnect". */
	private void leave(Minecraft mc) {
		//? if >=1.21.10 {
		mc.disconnectFromWorld(ClientLevel.DEFAULT_QUIT_MESSAGE);
		//?} else if >=1.21.6 {
		/*PauseScreen.disconnectFromWorld(mc, ClientLevel.DEFAULT_QUIT_MESSAGE);
		*///?} else {
		/*boolean local = mc.isLocalServer();
		ServerData server = mc.getCurrentServer();
		if (mc.level != null) mc.level.disconnect();
		boolean realm = Nav.wasRealm(mc, server);
		Nav.dropWorld(local);
		TitleScreen title = new TitleScreen();
		if (local) mc.setScreen(title);
		else if (realm) mc.setScreen(new RealmsMainScreen(title));
		else mc.setScreen(new JoinMultiplayerScreen(title));
		*///?}
	}

	@Override
	public boolean isPauseScreen() {
		Minecraft mc = Minecraft.getInstance();
		return mc.hasSingleplayerServer() && mc.getSingleplayerServer() != null && !mc.getSingleplayerServer().isPublished();
	}

	@Override
	protected boolean keepGameBackground() {
		return true;
	}

	@Override
	protected void paint(Gfx g, int mouseX, int mouseY, float delta) {
		int end = 16 + COL_W + 20;
		g.fill(0, 0, end, height, UiConfig.alpha(cfg.bg, 0.78f));
		g.gradientRight(end, 0, end + 50, height, UiConfig.alpha(cfg.bg, 0.78f), cfg.bg & 0x00FFFFFF);
	}

	@Override
	protected void paintOver(Gfx g, int mouseX, int mouseY, float delta) {
		Minecraft mc = Minecraft.getInstance();
		boolean ru = UiConfig.ru();
		int x = 16;
		g.fill(x, 14, x + 24, 16, cfg.accent);
		g.bigText(font, Component.literal(place(mc)), x, 22, 1.6f, cfg.text);
		long min = (System.currentTimeMillis() - STARTED) / 60000;
		String time = min >= 60 ? (min / 60) + (ru ? " ч " : " h ") + (min % 60) + (ru ? " мин" : " min") : min + (ru ? " мин" : " min");
		g.text(font, (ru ? "В игре: " : "Playing: ") + time, x, 42, cfg.muted);
		if (mc.player != null) {
			var p = mc.player.blockPosition();
			g.text(font, "XYZ: " + p.getX() + " / " + p.getY() + " / " + p.getZ(), x, 54, cfg.muted);
		}
		if (!cfg.name.isEmpty() && bottom + 22 < height) {
			String brand = cfg.name;
			g.text(font, brand, x, height - 16, UiConfig.alpha(cfg.text, 0.55f));
		}
	}

	private static String place(Minecraft mc) {
		try {
			if (mc.hasSingleplayerServer() && mc.getSingleplayerServer() != null) {
				String n = mc.getSingleplayerServer().getWorldData().getLevelName();
				return n.length() > 22 ? n.substring(0, 21) + "…" : n;
			}
			ServerData s = mc.getCurrentServer();
			if (s != null) return s.name.equals(s.ip) || s.name.isEmpty() ? s.ip : s.name;
		} catch (Throwable ignored) {
		}
		return "Minecraft";
	}
}
