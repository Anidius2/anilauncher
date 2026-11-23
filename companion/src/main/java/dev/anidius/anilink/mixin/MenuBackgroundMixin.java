package dev.anidius.anilink.mixin;

import dev.anidius.anilink.ui.BaseScreen;
import dev.anidius.anilink.ui.Gfx;
import dev.anidius.anilink.ui.Images;
import dev.anidius.anilink.ui.UiConfig;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.screens.Screen;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
//? if >=26.1 {
/*import net.minecraft.client.gui.GuiGraphicsExtractor;
*///?} else {
import net.minecraft.client.gui.GuiGraphics;
//?}

/**
 * Every other menu outside a world (options, mods, singleplayer, multiplayer, any mod's screen) gets the
 * build's art and the launcher's colours behind it instead of the game's panorama or dirt.
 */
@Mixin(Screen.class)
public abstract class MenuBackgroundMixin {
	private void anilink$art(Gfx g) {
		try {
			Screen self = (Screen) (Object) this;
			if (self instanceof BaseScreen) return;               // our own menus paint themselves
			if (Minecraft.getInstance().level != null) return;  // in a world the game keeps its blurred view
			UiConfig cfg = UiConfig.get();
			if (!cfg.menu) return;
			Images.Tex art = Images.get(cfg.art);
			if (art == null) return;
			g.cover(art, 0, 0, self.width, self.height, 1f);
			// the launcher's background colour over it, so lists and text stay readable on any picture
			g.fill(0, 0, self.width, self.height, UiConfig.alpha(cfg.bg, 0.62f));
			g.gradientDown(0, 0, self.width, 40, UiConfig.alpha(0xFF000000, 0.35f), 0x00000000);
			g.gradientDown(0, self.height - 40, self.width, self.height, 0x00000000, UiConfig.alpha(0xFF000000, 0.35f));
		} catch (Throwable ignored) {
			// the game's own background stays
		}
	}

	//? if >=26.1 {
	/*@Inject(method = "extractBackground", at = @At("TAIL"), require = 0)
	private void anilink$bg(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float delta, CallbackInfo ci) {
		anilink$art(new Gfx(graphics));
	}
	*///?} else if >=1.20.2 {
	@Inject(method = "renderBackground(Lnet/minecraft/client/gui/GuiGraphics;IIF)V", at = @At("TAIL"), require = 0)
	private void anilink$bg(GuiGraphics graphics, int mouseX, int mouseY, float delta, CallbackInfo ci) {
		anilink$art(new Gfx(graphics));
	}
	//?} else {
	/*@Inject(method = "renderBackground(Lnet/minecraft/client/gui/GuiGraphics;)V", at = @At("TAIL"), require = 0)
	private void anilink$bg(GuiGraphics graphics, CallbackInfo ci) {
		anilink$art(new Gfx(graphics));
	}
	*///?}
}
