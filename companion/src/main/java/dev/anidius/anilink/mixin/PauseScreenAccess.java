package dev.anidius.anilink.mixin;

import net.minecraft.client.gui.screens.PauseScreen;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;

/** 1.20.1 has no showsPauseMenu(): read the field (F3+Esc opens the pause screen without its menu). */
@Mixin(PauseScreen.class)
public interface PauseScreenAccess {
	@Accessor("showPauseMenu")
	boolean anilink$showsMenu();
}
