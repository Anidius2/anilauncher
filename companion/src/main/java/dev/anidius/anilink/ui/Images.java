package dev.anidius.anilink.ui;

import com.mojang.blaze3d.platform.NativeImage;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.resources.Identifier;

import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HashMap;
import java.util.Map;

/** PNG files from the launcher, uploaded once as game textures. */
public final class Images {
	public record Tex(Identifier id, int width, int height) {}

	private static final Map<Path, Tex> CACHE = new HashMap<>();
	private static int counter;

	private Images() {}

	/** Null when the file is missing or unreadable; never throws. */
	public static Tex get(Path file) {
		if (file == null) return null;
		if (CACHE.containsKey(file)) return CACHE.get(file);
		Tex tex = null;
		try (InputStream in = Files.newInputStream(file)) {
			NativeImage img = NativeImage.read(in);
			//? if >=1.21 {
			Identifier id = Identifier.fromNamespaceAndPath("anilink", "ui/" + (counter++));
			//?} else {
			/*Identifier id = new Identifier("anilink", "ui/" + (counter++));
			*///?}
			// Smooth (linear) filtering: big art is scaled to the window, nearest looks jagged.
			//? if >=26.3 {
			/*DynamicTexture dyn = new DynamicTexture(() -> "anilink ui", img) {
				{ sampler = com.mojang.blaze3d.systems.RenderSystem.getSamplerCache()
						.getClampToEdge(com.mojang.renderpearl.api.textures.FilterMode.LINEAR); }
			};
			*///?} else if >=1.21.11 {
			DynamicTexture dyn = new DynamicTexture(() -> "anilink ui", img) {
				{ sampler = com.mojang.blaze3d.systems.RenderSystem.getSamplerCache()
						.getClampToEdge(com.mojang.blaze3d.textures.FilterMode.LINEAR); }
			};
			//?} else if >=1.21.5 {
			/*DynamicTexture dyn = new DynamicTexture(() -> "anilink ui", img);
			dyn.setFilter(true, false);
			*///?} else {
			/*DynamicTexture dyn = new DynamicTexture(img);
			dyn.setFilter(true, false);
			*///?}
			Minecraft.getInstance().getTextureManager().register(id, dyn);
			tex = new Tex(id, img.getWidth(), img.getHeight());
		} catch (Throwable t) {
			System.out.println("[AniLink] image " + file + ": " + t);
		}
		CACHE.put(file, tex);
		return tex;
	}
}
