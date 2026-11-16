package dev.anidius.anilink;

/**
 * Shared start for every loader. Does nothing unless the game was started by AniLauncher with the
 * link switched on: the launcher passes the port and a one-time token as system properties.
 */
public final class Boot {
	private static boolean started;

	private Boot() {}

	public static synchronized void start() {
		if (started) return;
		started = true;
		String port = System.getProperty("anilink.port");
		String token = System.getProperty("anilink.token");
		if (port == null || token == null) return;
		try {
			Link.start(Integer.parseInt(port), token, System.getProperty("anilink.instance", ""));
		} catch (Throwable t) {
			Link.log("could not start: " + t);
		}
	}
}
