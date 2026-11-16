package dev.anidius.anilink;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import net.minecraft.client.Minecraft;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.net.InetAddress;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;

/**
 * Connection to the launcher: one line of JSON per message, both ways, over 127.0.0.1 only.
 * Everything that touches the game runs on the game thread through {@link Minecraft#execute}.
 */
public final class Link {
	private static volatile BufferedWriter out;
	private static final Object WRITE_LOCK = new Object();

	private Link() {}

	static void start(int port, String token, String instance) {
		Thread t = new Thread(() -> run(port, token, instance), "AniLink");
		t.setDaemon(true);
		t.start();
		ScheduledExecutorService timer = Executors.newSingleThreadScheduledExecutor(r -> {
			Thread th = new Thread(r, "AniLink-poll");
			th.setDaemon(true);
			return th;
		});
		timer.scheduleWithFixedDelay(() -> {
			Minecraft mc = Minecraft.getInstance();
			if (mc != null) mc.execute(Watch::poll);
		}, 3, 1, TimeUnit.SECONDS);
	}

	private static void run(int port, String token, String instance) {
		// The launcher may still be starting its side; try for a while, then give up quietly.
		for (int attempt = 0; attempt < 30; attempt++) {
			try (Socket s = new Socket(InetAddress.getLoopbackAddress(), port)) {
				s.setTcpNoDelay(true);
				BufferedReader in = new BufferedReader(new InputStreamReader(s.getInputStream(), StandardCharsets.UTF_8));
				out = new BufferedWriter(new OutputStreamWriter(s.getOutputStream(), StandardCharsets.UTF_8));
				JsonObject hello = new JsonObject();
				hello.addProperty("type", "hello");
				hello.addProperty("token", token);
				hello.addProperty("instance", instance);
				hello.addProperty("version", 1);
				send(hello);
				Watch.resend();
				String line;
				while ((line = in.readLine()) != null) {
					if (line.isBlank()) continue;
					JsonObject msg;
					try {
						msg = JsonParser.parseString(line).getAsJsonObject();
					} catch (Exception e) {
						continue;
					}
					Minecraft.getInstance().execute(() -> Actions.handle(msg));
				}
				return; // the launcher closed the connection on purpose
			} catch (Exception e) {
				out = null;
				try {
					Thread.sleep(2000);
				} catch (InterruptedException ie) {
					return;
				}
			}
		}
	}

	public static void send(JsonObject msg) {
		BufferedWriter w = out;
		if (w == null) return;
		synchronized (WRITE_LOCK) {
			try {
				w.write(msg.toString());
				w.write('\n');
				w.flush();
			} catch (Exception e) {
				out = null;
			}
		}
	}

	static void log(String text) {
		System.out.println("[AniLink] " + text);
	}
}
