import net.fabricmc.mappingio.adapter.MappingSourceNsSwitch;
import net.fabricmc.mappingio.format.proguard.ProGuardFileReader;
import net.fabricmc.mappingio.format.srg.TsrgFileReader;
import net.fabricmc.mappingio.tree.MappingTree;
import net.fabricmc.mappingio.tree.MemoryMappingTree;
import net.fabricmc.tinyremapper.IMappingProvider;
import net.fabricmc.tinyremapper.OutputConsumerPath;
import net.fabricmc.tinyremapper.TinyRemapper;
import net.fabricmc.tinyremapper.extension.mixin.MixinExtension;

import java.io.Reader;
import java.io.StringReader;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.zip.ZipFile;

/**
 * Turns classes compiled against Mojang's names into SRG names (Forge 1.20.1–1.20.4 and NeoForge
 * 1.20.2–1.20.4 run on those). Mapping: Mojang names → obfuscated (client.txt) → SRG (mcp_config).
 * Mixin annotations are rewritten in place, so no refmap is needed.
 *
 * args: in.jar out.jar client.txt mcp_config.zip minecraft-mojang-names.jar
 */
public final class Reobf {
	public static void main(String[] a) throws Exception {
		Path in = Path.of(a[0]), out = Path.of(a[1]), client = Path.of(a[2]), mcp = Path.of(a[3]), game = Path.of(a[4]);

		MemoryMappingTree named = new MemoryMappingTree();
		try (Reader r = Files.newBufferedReader(client)) {
			ProGuardFileReader.read(r, "named", "obf", named);
		}
		MemoryMappingTree tree = new MemoryMappingTree();
		named.accept(new MappingSourceNsSwitch(tree, "obf"));
		String tsrg;
		try (ZipFile z = new ZipFile(mcp.toFile())) {
			tsrg = new String(z.getInputStream(z.getEntry("config/joined.tsrg")).readAllBytes());
		}
		TsrgFileReader.read(new StringReader(tsrg), tree);

		int n = tree.getNamespaceId("named"), s = tree.getNamespaceId("srg");
		if (n < 0 || s < 0) throw new IllegalStateException("namespaces: " + tree.getDstNamespaces());
		int[] count = new int[2];
		IMappingProvider mappings = acceptor -> {
			for (MappingTree.ClassMapping c : tree.getClasses()) {
				String owner = c.getName(n);
				if (owner == null) continue;
				for (MappingTree.MethodMapping m : c.getMethods()) {
					String from = m.getName(n), to = m.getName(s), desc = m.getDesc(n);
					if (from == null || to == null || desc == null || from.equals(to)) continue;
					acceptor.acceptMethod(new IMappingProvider.Member(owner, from, desc), to);
					count[0]++;
				}
				for (MappingTree.FieldMapping f : c.getFields()) {
					String from = f.getName(n), to = f.getName(s), desc = f.getDesc(n);
					if (from == null || to == null || desc == null || from.equals(to)) continue;
					acceptor.acceptField(new IMappingProvider.Member(owner, from, desc), to);
					count[1]++;
				}
			}
		};

		TinyRemapper remapper = TinyRemapper.newRemapper()
			.withMappings(mappings)
			.extension(new MixinExtension())
			.rebuildSourceFilenames(true)
			.build();
		Files.deleteIfExists(out);
		try (OutputConsumerPath sink = new OutputConsumerPath.Builder(out).build()) {
			sink.addNonClassFiles(in);
			remapper.readClassPath(game);
			remapper.readInputs(in);
			remapper.apply(sink);
		} finally {
			remapper.finish();
		}
		System.out.println("reobf: " + count[0] + " methods, " + count[1] + " fields -> " + out);
	}
}
