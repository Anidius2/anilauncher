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
		super.render(graphics, mouseX, mouseY, delta);
		paintOver(new Gfx(graphics), mouseX, mouseY, delta);
	}

	@Override
	public void renderBackground(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
		if (keepGameBackground()) super.renderBackground(graphics, mouseX, mouseY, delta);
		paint(new Gfx(graphics), mouseX, mouseY, delta);
	}
	//?}
}
