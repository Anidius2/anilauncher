package dev.anidius.anilink.ui;

import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.screens.TitleScreen;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.client.gui.screens.multiplayer.JoinMultiplayerScreen;
import net.minecraft.client.gui.screens.worldselection.SelectWorldScreen;
import net.minecraft.network.chat.Component;

/** The main menu in the launcher's style: the build's art, its name and one button to carry on. */
public final class MenuScreen extends BaseScreen {
	private static final int COL_W = 180;
	private boolean continueWorld;
	private String continueTarget = "";
	private int continueY = -1;
	private int top = 24;

	public MenuScreen() {
		super(Component.literal("AniLauncher"));
	}

	@Override
	protected void init() {
		Minecraft mc = Minecraft.getInstance();
		pickContinue(mc);
		continueY = -1;
		int gap = 4, h = 20;
		boolean ru = UiConfig.ru();
		int half = (COL_W - gap) / 2;

		// what else the game's title screen has: Realms, language, accessibility and other mods' buttons
		List<AbstractWidget> wide = new ArrayList<>(), icons = new ArrayList<>();
		foreignButtons(new TitleScreen(), new String[] {"menu.singleplayer", "menu.multiplayer", "menu.options", "menu.quit", "modmenu", "fml.menu"}, wide, icons);
		// Options, Mods and the wide ones go two per row
		List<AbstractWidget> pairs = new ArrayList<>();
		pairs.add(button(Component.translatable("menu.options"), 0, 0, half, h, () -> Nav.open(Nav.options(this, false))));
		if (Nav.hasModMenu()) pairs.add(button(Component.literal(ru ? "Моды" : "Mods"), 0, 0, half, h, () -> Nav.openMods(this)));
		for (AbstractWidget w : wide) pairs.add(addRenderableWidget(w));
		// icons in rows under "Classic menu / Quit" (beside them the labels would not fit)
		int beside = 0, besideW = 0;
		int iconRows = 0;
		for (int i = beside, ix = COL_W; i < icons.size(); i++) {
			int w = icons.get(i).getWidth();
			if (ix + w > COL_W) {
				iconRows++;
				ix = 0;
			}
			ix += w + gap;
		}
		int rows = 2 + (pairs.size() + 1) / 2 + 1 + iconRows;
		int menuH = rows * (h + gap) + (continueTarget.isEmpty() ? 0 : h + gap + 4);
		// move up when it would run into the player name at the bottom
		top = Math.max(8, Math.min(Math.max(24, height / 2 - 92), height - 44 - menuH - 48));

		int x = left();
		int y = buttonsTop();
		if (!continueTarget.isEmpty()) {
			String label = (ru ? "▶ Продолжить: " : "▶ Continue: ") + shorten(continueLabel(), 22);
			button(Component.literal(label), x, y, COL_W, h, this::carryOn);
			continueY = y;
			y += h + gap + 4;
		}
		button(Component.translatable("menu.singleplayer"), x, y, COL_W, h, () -> Nav.open(new SelectWorldScreen(this)));
		y += h + gap;
		button(Component.translatable("menu.multiplayer"), x, y, COL_W, h, () -> Nav.open(new JoinMultiplayerScreen(this)));
		y += h + gap;
		for (int i = 0; i < pairs.size(); i++) {
			boolean right = i % 2 == 1;
			boolean alone = i == pairs.size() - 1 && !right;
			AbstractWidget b = pairs.get(i);
			b.setX(right ? x + half + gap : x);
			b.setY(y);
			b.setWidth(alone ? COL_W : right ? COL_W - half - gap : half);
			if (right || alone) y += h + gap;
		}
		int rest = COL_W - besideW;
		int classicW = (rest - gap) / 2;
		button(Component.literal(ru ? "Обычное меню" : "Classic menu"), x, y, classicW, h, Nav::classicTitle);
		// "Выйти из игры" does not fit half a column
		Component quit = ru ? Component.literal("Выйти") : Component.translatable("menu.quit");
		button(quit, x + classicW + gap, y, rest - classicW - gap, h, () -> Minecraft.getInstance().stop());
		int ix = x + rest + gap;
		for (int i = 0; i < beside; i++) {
			AbstractWidget b = icons.get(i);
			place(b, ix, y + (h - b.getHeight()) / 2, b.getWidth());
			ix += b.getWidth() + gap;
		}
		ix = x + COL_W;
		for (int i = beside; i < icons.size(); i++) {
			AbstractWidget b = icons.get(i);
			if (ix + b.getWidth() > x + COL_W) {
				ix = x;
				y += h + gap;
			}
			place(b, ix, y + (h - b.getHeight()) / 2, b.getWidth());
			ix += b.getWidth() + gap;
		}
	}

	private int left() {
		return Math.max(16, width / 14);
	}

	private int titleTop() {
		return top;
	}

	private int buttonsTop() {
		return titleTop() + 48;
	}

	/** The world or server played last, whichever is newer and still there. */
	private void pickContinue(Minecraft mc) {
		continueTarget = "";
		boolean worldOk = !cfg.worldId.isEmpty() && worldExists(mc, cfg.worldId);
		boolean serverOk = !cfg.server.isEmpty();
		if (worldOk && (!serverOk || cfg.worldTime >= cfg.serverTime)) {
			continueWorld = true;
			continueTarget = cfg.worldId;
		} else if (serverOk) {
			continueWorld = false;
			continueTarget = cfg.server;
		}
	}

	private static boolean worldExists(Minecraft mc, String id) {
		try {
			return mc.getLevelSource().levelExists(id);
		} catch (Throwable t) {
			return false;
		}
	}

	private String continueLabel() {
		if (continueWorld) return cfg.worldName.isEmpty() ? cfg.worldId : cfg.worldName;
		return continueTarget;
	}

	private static String shorten(String s, int max) {
		return s.length() <= max ? s : s.substring(0, max - 1) + "…";
	}

	private void carryOn() {
		Minecraft mc = Minecraft.getInstance();
		if (continueWorld) {
			Nav.openWorld(this, continueTarget);
		} else {
			Nav.connect(this, continueTarget);
		}
	}

	@Override
	protected boolean keepGameBackground() {
		// no art: the game's panorama stays, we only add our panel
		return Images.get(cfg.art) == null;
	}

	@Override
	protected void paint(Gfx g, int mouseX, int mouseY, float delta) {
		Images.Tex art = Images.get(cfg.art);
		if (art != null) g.cover(art, 0, 0, width, height, 1f);
		// darken the left side so the menu reads well on any picture
		int panelEnd = left() + COL_W + Math.max(60, width / 5);
		g.fill(0, 0, left() + COL_W + 24, height, UiConfig.alpha(cfg.bg, 0.72f));
		g.gradientRight(left() + COL_W + 24, 0, panelEnd, height, UiConfig.alpha(cfg.bg, 0.72f), cfg.bg & 0x00FFFFFF);
		g.gradientDown(0, height - 70, width, height, 0x00000000, UiConfig.alpha(0xFF000000, 0.55f));
	}

	@Override
	protected void paintOver(Gfx g, int mouseX, int mouseY, float delta) {
		int x = left();
		int y = titleTop();
		String name = cfg.name.isEmpty() ? "Minecraft" : cfg.name;
		float scale = Math.min(2.5f, (COL_W + 60) / (float) Math.max(1, font.width(name)));
		g.fill(x, y - 6, x + 28, y - 4, cfg.accent);
		g.bigText(font, Component.literal(name), x, y, Math.max(1.2f, scale), cfg.text);
		if (!cfg.subtitle.isEmpty()) g.text(font, cfg.subtitle, x, y + 26, cfg.muted);

		// the main button gets the launcher's accent
		if (continueY >= 0) g.fill(x - 4, continueY + 2, x - 2, continueY + 18, cfg.accent);

		// player at the bottom left
		int by = height - 34;
		Images.Tex head = Images.get(cfg.head);
		int tx = x;
		if (head != null) {
			g.fill(x - 1, by - 1, x + 21, by + 21, UiConfig.alpha(cfg.accent, 0.9f));
			g.image(head, x, by, 20, 20, 1f);
			tx = x + 26;
		}
		if (!cfg.nick.isEmpty()) {
			g.text(font, cfg.nick, tx, by + 1, cfg.text);
			String acc = cfg.licensed ? UiConfig.tr("Лицензия", "Licensed") : UiConfig.tr("Офлайн", "Offline");
			g.text(font, acc, tx, by + 11, cfg.muted);
		}
		String brand = "AniLauncher";
		g.text(font, brand, width - font.width(brand) - 8, height - 14, UiConfig.alpha(cfg.text, 0.6f));
	}

	@Override
	public boolean shouldCloseOnEsc() {
		return false;
	}
}
