package dev.anidius.anilink;

/**
 * Entry point for both Forge and NeoForge: each loader only sees its own @Mod annotation.
 * Does nothing on a dedicated server (no client classes there).
 */
@net.minecraftforge.fml.common.Mod("anilink")
@net.neoforged.fml.common.Mod("anilink")
public final class AniLinkForge {
	public AniLinkForge() {
		try {
			Class.forName("net.minecraft.client.Minecraft", false, AniLinkForge.class.getClassLoader());
		} catch (Throwable notClient) {
			return;
		}
		Boot.start();
	}
}
