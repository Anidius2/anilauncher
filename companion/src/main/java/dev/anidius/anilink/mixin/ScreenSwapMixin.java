package dev.anidius.anilink.mixin;

import dev.anidius.anilink.ui.Swap;
import net.minecraft.client.gui.screens.Screen;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.ModifyVariable;

//? if >=26.2 {
/*@Mixin(net.minecraft.client.gui.Gui.class)
*///?} else {
@Mixin(net.minecraft.client.Minecraft.class)
//?}
public abstract class ScreenSwapMixin {
	@ModifyVariable(method = "setScreen", at = @At("HEAD"), argsOnly = true, require = 0)
	private Screen anilink$swap(Screen screen) {
		return Swap.swap(screen);
	}
}
