package dev.anidius.anilink.mixin;

import dev.anidius.anilink.ui.Gfx;
import dev.anidius.anilink.ui.LoadingArt;
import dev.anidius.anilink.ui.UiConfig;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.screens.LoadingOverlay;
import net.minecraft.util.Mth;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
//? if >=26.1 {
/*import net.minecraft.client.gui.GuiGraphicsExtractor;
*///?} else {
import net.minecraft.client.gui.GuiGraphics;
//?}

@Mixin(LoadingOverlay.class)
public abstract class LoadingOverlayMixin {
	@Shadow private float currentProgress;
	@Shadow private long fadeOutStart;
	@Shadow private long fadeInStart;
	@Shadow @Final private boolean fadeIn;

	// Once loading is done, our art would cross-fade badly with the game's red logo screen. The menu
	// behind has the same art anyway: end the overlay at once instead of fading.
	//? if >=26.1 {
	/*@Inject(method = "extractRenderState", at = @At("HEAD"), require = 0)
	private void anilink$skipFade(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float delta, CallbackInfo ci) {
		anilink$skip();
	}
	*///?} else {
	@Inject(method = "render", at = @At("HEAD"), require = 0)
	private void anilink$skipFade(GuiGraphics graphics, int mouseX, int mouseY, float delta, CallbackInfo ci) {
		anilink$skip();
	}
	//?}

	private long anilink$lastLog;

	/** The game's own clock for the fade (not the same as System.nanoTime on every version). */
	private static long anilink$millis() {
		//? if >=1.21.11 {
		return net.minecraft.util.Util.getMillis();
		//?} else {
		/*return net.minecraft.Util.getMillis();
		*///?}
	}

	private void anilink$skip() {
		if (!UiConfig.get().loading || fadeOutStart <= -1L) return;
		long now = anilink$millis();
		if (now - fadeOutStart < 2000L) {
			if (Boolean.getBoolean("anilink.debug")) System.out.println("[AniLink] skip fade " + (now - fadeOutStart));
			fadeOutStart = now - 2001L;
		}
	}

	//? if >=26.1 {
	/*@Inject(method = "extractRenderState", at = @At("TAIL"), require = 0)
	private void anilink$art(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float delta, CallbackInfo ci) {
		anilink$draw(new Gfx(graphics));
	}
	*///?} else {
	@Inject(method = "render", at = @At("TAIL"), require = 0)
	private void anilink$art(GuiGraphics graphics, int mouseX, int mouseY, float delta, CallbackInfo ci) {
		anilink$draw(new Gfx(graphics));
	}
	//?}

	private void anilink$draw(Gfx g) {
		// the same fade the game uses for its own loading screen
		long now = anilink$millis();
		float out = fadeOutStart > -1L ? (now - fadeOutStart) / 1000.0F : -1.0F;
		float in = fadeInStart > -1L ? (now - fadeInStart) / 500.0F : -1.0F;
		float alpha;
		if (out >= 1.0F) alpha = 1.0F - Mth.clamp(out - 1.0F, 0.0F, 1.0F);
		else if (fadeIn) alpha = Mth.clamp(in, 0.0F, 1.0F);
		else alpha = 1.0F;
		Minecraft mc = Minecraft.getInstance();
		if (Boolean.getBoolean("anilink.debug") && now - anilink$lastLog > 250) {
			anilink$lastLog = now;
			System.out.println("[AniLink] overlay " + this.hashCode() + " alpha=" + alpha + " out=" + out + " in=" + in + " fadeIn=" + fadeIn + " p=" + currentProgress);
		}
		LoadingArt.draw(g, mc.getWindow().getGuiScaledWidth(), mc.getWindow().getGuiScaledHeight(), currentProgress, alpha);
	}
}
