package dev.anidius.anilink;

import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.components.toasts.SystemToast;
import net.minecraft.network.chat.Component;

/** The few calls that differ between Minecraft versions (Fabric build). */
final class Compat {
	//? if >=1.20.3 {
	private static final SystemToast.SystemToastId TOAST = new SystemToast.SystemToastId(6000L);
	//?} else {
	/*private static final SystemToast.SystemToastIds TOAST = SystemToast.SystemToastIds.PERIODIC_NOTIFICATION;
	*///?}

	private Compat() {}

	static void toast(String title, String text) {
		Minecraft mc = Minecraft.getInstance();
		Component t = Component.literal(title);
		Component d = text.isEmpty() ? null : Component.literal(text);
		//? if >=26.2 {
		/*SystemToast.addOrUpdate(mc.gui.toastManager(), TOAST, t, d);
		*///?} else if >=1.21.2 {
		SystemToast.addOrUpdate(mc.getToastManager(), TOAST, t, d);
		//?} else {
		/*SystemToast.addOrUpdate(mc.getToasts(), TOAST, t, d);
		*///?}
	}

	static void resizeGui() {
		//? if >=26.1 {
		/*Minecraft.getInstance().resizeGui();
		*///?} else {
		Minecraft.getInstance().resizeDisplay();
		//?}
	}
}
