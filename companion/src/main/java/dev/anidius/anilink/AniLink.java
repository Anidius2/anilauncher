package dev.anidius.anilink;

import net.fabricmc.api.ClientModInitializer;

/** Fabric and Quilt entry point. */
public final class AniLink implements ClientModInitializer {
	@Override
	public void onInitializeClient() {
		Boot.start();
	}
}
