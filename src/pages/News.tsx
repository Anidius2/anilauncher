import { api, errText, type NewsItem } from "../api";
import { useStore } from "../store";
import { sfx } from "../sounds";

function Card({ n }: { n: NewsItem }) {
  const { toast, settings } = useStore();
  const date = n.date ? new Date(n.date).toLocaleDateString(settings.lang === "ru" ? "ru-RU" : "en-GB") : "";
  return (
    <button
      className="card news-card"
      onMouseEnter={sfx.hover}
      onClick={() => n.link && api.openUrl(n.link).catch((e) => toast(errText(e), "error"))}
      disabled={!n.link}
    >
      <span className="news-card__img">{n.image && <img className="art-fill" src={n.image} alt="" loading="lazy" />}</span>
      <span className="news-card__text">
        <span className="news-card__tag">
          {n.tag}
          {date ? ` · ${date}` : ""}
        </span>
        <span className="news-card__title">{n.title}</span>
        {n.text && <span className="muted small clamp">{n.text}</span>}
      </span>
    </button>
  );
}

export function News() {
  const { t, feed, feedError } = useStore();
  return (
    <div className="page">
      <div className="page-head">
        <h1>{t("news.title")}</h1>
      </div>
      {feedError && <p className="empty">{t("news.error")}</p>}
      <div className="grid grid--3">
        {(feed?.news ?? []).map((n) => (
          <Card key={n.id} n={n} />
        ))}
      </div>
      <h2 className="section-title">{t("news.patch")}</h2>
      <div className="grid grid--4">
        {(feed?.patchNotes ?? []).slice(0, 24).map((n) => (
          <Card key={n.id} n={n} />
        ))}
      </div>
    </div>
  );
}
