import { open } from "@tauri-apps/plugin-dialog";
import { api, fileSrc, errText, type Art } from "../api";
import { useStore } from "../store";
import { Modal } from "../components/Bits";
import { Icon } from "../components/Icon";
import { sfx } from "../sounds";

/** Pick a build's art: any official Mojang picture from the feed, or an own file. */
export function ArtPicker({ onPick, onClose }: { onPick: (art: Art | null) => void; onClose: () => void }) {
  const { t, feed, feedError, toast } = useStore();
  const images = Array.from(
    new Map(
      [...(feed?.news ?? []), ...(feed?.patchNotes ?? [])]
        .filter((n) => n.image)
        .map((n) => [n.image as string, n.title]),
    ),
  );

  const pickFile = async () => {
    try {
      const file = await open({
        multiple: false,
        filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }],
      });
      if (typeof file !== "string") return;
      const stored = await api.importMedia(file);
      sfx.success();
      onPick({ kind: "file", value: stored });
    } catch (e) {
      toast(errText(e), "error");
    }
  };

  return (
    <Modal title={t("art.title")} onClose={onClose} wide>
      <div className="art-actions">
        <button className="btn btn--primary" onClick={pickFile}>
          <Icon name="image" /> {t("art.own")}
        </button>
        <button className="btn" onClick={() => onPick(null)}>
          {t("art.none")}
        </button>
      </div>
      {!!feed?.art?.length && (
        <>
          <h3 className="section-title">{t("art.big")}</h3>
          <div className="art-grid art-grid--wide">
            {feed.art.map((p) => (
              <button
                key={p}
                className="art-tile"
                onMouseEnter={sfx.hover}
                onClick={async () => {
                  sfx.click();
                  try {
                    // keep an own copy: the cache may be cleared
                    onPick({ kind: "file", value: await api.importMedia(p) });
                  } catch (e) {
                    toast(errText(e), "error");
                  }
                }}
              >
                <img src={fileSrc(p) ?? ""} alt="" loading="lazy" />
              </button>
            ))}
          </div>
        </>
      )}
      <h3 className="section-title">{t("art.mojang")}</h3>
      {!feed && !feedError && <p className="muted">{t("art.loading")}</p>}
      {feedError && <p className="muted">{t("news.error")}</p>}
      <div className="art-grid">
        {images.map(([url, title]) => (
          <button
            key={url}
            className="art-tile"
            title={title}
            onMouseEnter={sfx.hover}
            onClick={() => {
              sfx.click();
              onPick({ kind: "url", value: url });
            }}
          >
            <img src={url} alt={title} loading="lazy" />
          </button>
        ))}
      </div>
    </Modal>
  );
}
