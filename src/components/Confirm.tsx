import { useEffect, useRef, useState } from "react";
import { useStore } from "../store";

/**
 * In-app yes/no question. Replaces the system message box, which on Windows could open
 * behind the launcher's frameless window and leave it looking frozen.
 * Same call shape as the dialog plugin: `await confirm(text, { kind: "warning" })`.
 */
type Ask = { text: string; danger: boolean; done: (ok: boolean) => void };
let push: ((a: Ask) => void) | null = null;

export function confirm(text: string, opts?: { title?: string; kind?: "info" | "warning" | "error" }): Promise<boolean> {
  return new Promise((done) => {
    if (!push) return done(window.confirm(text));
    push({ text, danger: opts?.kind === "warning" || opts?.kind === "error", done });
  });
}

export function ConfirmHost() {
  const { settings } = useStore();
  const [ask, setAsk] = useState<Ask | null>(null);
  const yes = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    push = (a) => setAsk(a);
    return () => void (push = null);
  }, []);
  useEffect(() => {
    if (!ask) return;
    yes.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") answer(false);
      if (e.key === "Enter") answer(true);
      e.stopPropagation();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  });
  if (!ask) return null;
  const ru = settings.lang === "ru";
  function answer(ok: boolean) {
    ask?.done(ok);
    setAsk(null);
  }
  return (
    <div className="modal-back confirm-back" onMouseDown={(e) => e.target === e.currentTarget && answer(false)}>
      <div className="modal confirm" role="alertdialog" aria-modal="true" aria-label={ask.text}>
        <p className="confirm__text">{ask.text}</p>
        <div className="confirm__actions">
          <button className="btn" onClick={() => answer(false)}>
            {ru ? "Отмена" : "Cancel"}
          </button>
          <button ref={yes} className={`btn ${ask.danger ? "btn--danger-solid" : "btn--primary"}`} onClick={() => answer(true)}>
            {ru ? "Да" : "Yes"}
          </button>
        </div>
      </div>
    </div>
  );
}
