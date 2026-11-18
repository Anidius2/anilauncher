package dev.anidius.anilink.ui;

import net.minecraft.client.gui.Font;
import net.minecraft.network.chat.Component;
//? if >=26.1 {
/*import net.minecraft.client.gui.GuiGraphicsExtractor;
*///?} else {
import net.minecraft.client.gui.GuiGraphics;
//?}
//? if >=1.21.6 {
import net.minecraft.client.renderer.RenderPipelines;
//?} else if >=1.21.2 {
/*import net.minecraft.client.renderer.RenderType;
*///?}

/** The drawing calls the menus need, written once for every Minecraft version. */
public final class Gfx {
	//? if >=26.1 {
	/*public final GuiGraphicsExtractor g;

	public Gfx(GuiGraphicsExtractor g) {
		this.g = g;
	}
	*///?} else {
	public final GuiGraphics g;

	public Gfx(GuiGraphics g) {
		this.g = g;
	}
	//?}

	public void fill(int x0, int y0, int x1, int y1, int argb) {
		if ((argb >>> 24) == 0) return;
		g.fill(x0, y0, x1, y1, argb);
	}

	public void gradientDown(int x0, int y0, int x1, int y1, int top, int bottom) {
		g.fillGradient(x0, y0, x1, y1, top, bottom);
	}

	/** Left-to-right gradient, drawn as thin strips (the game only has a vertical one). */
	public void gradientRight(int x0, int y0, int x1, int y1, int left, int right) {
		int w = Math.max(1, x1 - x0);
		int step = 2;
		for (int x = x0; x < x1; x += step) {
			float t = (x - x0) / (float) w;
			int a = Math.round(((left >>> 24) & 0xFF) * (1 - t) + ((right >>> 24) & 0xFF) * t);
			int rgb = UiConfig.mix(left, right, t) & 0xFFFFFF;
			fill(x, y0, Math.min(x + step, x1), y1, (a << 24) | rgb);
		}
	}

	/** A 1px frame. */
	public void frame(int x0, int y0, int x1, int y1, int argb) {
		fill(x0, y0, x1, y0 + 1, argb);
		fill(x0, y1 - 1, x1, y1, argb);
		fill(x0, y0 + 1, x0 + 1, y1 - 1, argb);
		fill(x1 - 1, y0 + 1, x1, y1 - 1, argb);
	}

	public void text(Font font, Component c, int x, int y, int argb, boolean shadow) {
		//? if >=26.1 {
		/*g.text(font, c, x, y, argb, shadow);
		*///?} else {
		g.drawString(font, c, x, y, argb, shadow);
		//?}
	}

	public void text(Font font, String s, int x, int y, int argb) {
		text(font, Component.literal(s), x, y, argb, true);
	}

	/** Text scaled up around its top-left corner. */
	public void bigText(Font font, Component c, int x, int y, float scale, int argb) {
		//? if >=1.21.6 {
		g.pose().pushMatrix();
		g.pose().translate(x, y);
		g.pose().scale(scale, scale);
		text(font, c, 0, 0, argb, true);
		g.pose().popMatrix();
		//?} else {
		/*g.pose().pushPose();
		g.pose().translate(x, y, 0);
		g.pose().scale(scale, scale, 1);
		text(font, c, 0, 0, argb, true);
		g.pose().popPose();
		*///?}
	}

	/** Draws a texture so it fills the box (cropping the overflow), with transparency. */
	public void cover(Images.Tex t, int x, int y, int w, int h, float alpha) {
		if (t == null || w <= 0 || h <= 0) return;
		float scale = Math.max(w / (float) t.width(), h / (float) t.height());
		int rw = Math.min(t.width(), Math.round(w / scale));
		int rh = Math.min(t.height(), Math.round(h / scale));
		int u = (t.width() - rw) / 2;
		int v = (t.height() - rh) / 2;
		image(t, x, y, w, h, u, v, rw, rh, alpha);
	}

	/** Draws the whole texture into the box. */
	public void image(Images.Tex t, int x, int y, int w, int h, float alpha) {
		if (t == null) return;
		image(t, x, y, w, h, 0, 0, t.width(), t.height(), alpha);
	}

	private void image(Images.Tex t, int x, int y, int w, int h, int u, int v, int rw, int rh, float alpha) {
		int color = UiConfig.alpha(0xFFFFFFFF, alpha);
		//? if >=1.21.6 {
		g.blit(RenderPipelines.GUI_TEXTURED, t.id(), x, y, u, v, w, h, rw, rh, t.width(), t.height(), color);
		//?} else if >=1.21.2 {
		/*g.blit(RenderType::guiTextured, t.id(), x, y, u, v, w, h, rw, rh, t.width(), t.height(), color);
		*///?} else {
		/*g.setColor(1f, 1f, 1f, alpha);
		com.mojang.blaze3d.systems.RenderSystem.enableBlend();
		g.blit(t.id(), x, y, w, h, u, v, rw, rh, t.width(), t.height());
		g.setColor(1f, 1f, 1f, 1f);
		*///?}
	}
}
