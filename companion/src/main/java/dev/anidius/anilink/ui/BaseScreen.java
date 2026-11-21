package dev.anidius.anilink.ui;

import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
//? if >=26.1 {
/*import net.minecraft.client.gui.GuiGraphicsExtractor;
*///?} else {
import net.minecraft.client.gui.GuiGraphics;
//?}

/** A screen that paints its own background and decorations; widgets are the game's own buttons. */
public abstract class BaseScreen extends Screen {
	protected final UiConfig cfg = UiConfig.get();

	protected BaseScreen(Component title) {
		super(title);
	}

	/** Under the buttons. */
	protected abstract void paint(Gfx g, int mouseX, int mouseY, float delta);

	/** Over the buttons. */
	protected void paintOver(Gfx g, int mouseX, int mouseY, float delta) {}

	/** True: the game's own background (panorama or the blurred world) stays under our paint. */
	protected boolean keepGameBackground() {
		return false;
	}

	protected void place(net.minecraft.client.gui.components.AbstractWidget b, int x, int y, int w) {
		b.setX(x);
		b.setY(y);
		b.setWidth(w);
		addRenderableWidget(b);
	}

	/**
	 * Buttons other mods (Figura, voice chat, replay...) put into one of the game's own screens: that screen is
	 * built off screen so the mods add their buttons as usual, then those buttons move here. They still work as
	 * before: what they open comes back to the game's screen, and the game shows ours in its place again.
	 * Buttons whose text comes from a key starting with one of {@code ours} are left out (we have our own).
	 */
	protected void foreignButtons(Screen vanilla, String[] ours, java.util.List<net.minecraft.client.gui.components.AbstractWidget> wide,
			java.util.List<net.minecraft.client.gui.components.AbstractWidget> icons) {
		try {
			//? if >=1.21.11 {
			vanilla.init(width, height);
			//?} else {
			/*vanilla.init(net.minecraft.client.Minecraft.getInstance(), width, height);
			*///?}
			for (var child : vanilla.children()) {
				if (!(child instanceof net.minecraft.client.gui.components.AbstractButton b) || !b.visible) continue;
				if (b instanceof net.minecraft.client.gui.components.PlainTextButton) continue; // "Copyright Mojang"
				if (b.getMessage().getContents() instanceof net.minecraft.network.chat.contents.TranslatableContents t) {
					String k = t.getKey();
					boolean mine = false;
					for (String o : ours) mine |= k.startsWith(o);
					if (mine) continue;
				}
				(b.getWidth() <= 24 ? icons : wide).add(b);
			}
		} catch (Throwable ignored) {
			// a mod's button that can't be moved stays in the classic menu
		}
	}

	protected Button button(Component text, int x, int y, int w, int h, Runnable action) {
		return addRenderableWidget(Button.builder(text, b -> action.run()).bounds(x, y, w, h).build());
	}

	// The game calls the background first, then render() draws the widgets: we paint inside the background.
	//? if >=26.1 {
	/*@Override
	public void extractRenderState(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float delta) {
		super.extractRenderState(graphics, mouseX, mouseY, delta);
		paintOver(new Gfx(graphics), mouseX, mouseY, delta);
	}

	@Override
	public void extractBackground(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float delta) {
		if (keepGameBackground()) super.extractBackground(graphics, mouseX, mouseY, delta);
		paint(new Gfx(graphics), mouseX, mouseY, delta);
	}
	*///?} else {
	@Override
	public void render(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
		//? if <1.20.2 {
		/*// 1.20.1: each screen draws its own background before the widgets
		if (keepGameBackground()) renderBackground(graphics);
		paint(new Gfx(graphics), mouseX, mouseY, delta);
		*///?}
		super.render(graphics, mouseX, mouseY, delta);
		paintOver(new Gfx(graphics), mouseX, mouseY, delta);
	}

	//? if >=1.20.2 {
	@Override
	public void renderBackground(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
		if (keepGameBackground()) super.renderBackground(graphics, mouseX, mouseY, delta);
		paint(new Gfx(graphics), mouseX, mouseY, delta);
	}
	//?}
	//?}
}
