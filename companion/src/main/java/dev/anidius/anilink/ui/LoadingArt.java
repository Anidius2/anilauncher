package dev.anidius.anilink.ui;

import net.minecraft.util.Mth;

/** Draws the build's art and a progress bar over the game's loading screen. */
public final class LoadingArt {
	private LoadingArt() {}

	public static void draw(Gfx g, int width, int height, float progress, float alpha) {
		UiConfig cfg = UiConfig.get();
		if (!cfg.loading || alpha <= 0.01f) return;
		try {
			Images.Tex art = Images.get(cfg.art);
			g.fill(0, 0, width, height, UiConfig.alpha(cfg.bg, alpha));
			if (art != null) {
				g.cover(art, 0, 0, width, height, alpha);
				g.gradientDown(0, height / 2, width, height, 0x00000000, UiConfig.alpha(0xFF000000, 0.65f * alpha));
			}
			int barW = Math.min(width - 40, Math.max(120, width / 2));
			int x0 = (width - barW) / 2;
			int y0 = height - Math.max(24, height / 9);
			g.fill(x0 - 1, y0 - 1, x0 + barW + 1, y0 + 5, UiConfig.alpha(0xFF000000, 0.5f * alpha));
			g.fill(x0, y0, x0 + barW, y0 + 4, UiConfig.alpha(cfg.panel, alpha));
			int filled = Math.round(barW * Mth.clamp(progress, 0f, 1f));
			g.fill(x0, y0, x0 + filled, y0 + 4, UiConfig.alpha(cfg.accent, alpha));
		} catch (Throwable t) {
			System.out.println("[AniLink] loading: " + t);
		}
	}
}
