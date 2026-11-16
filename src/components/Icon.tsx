const paths: Record<string, JSX.Element> = {
  home: <path d="M3 11l9-7 9 7v9a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z" />,
  builds: (
    <>
      <path d="M12 3l9 5-9 5-9-5z" />
      <path d="M3 13l9 5 9-5" />
      <path d="M3 17.5l9 5 9-5" />
    </>
  ),
  mods: <path d="M10 3h4v3a2 2 0 1 0 4 0V3h3v7h-3a2 2 0 1 0 0 4h3v7h-7v-3a2 2 0 1 0-4 0v3H3v-7h3a2 2 0 1 0 0-4H3V3z" />,
  skins: <path d="M8 3l4 2 4-2 5 4-3 4-2-1v11H8V10l-2 1-3-4z" />,
  shots: (
    <>
      <rect x="3" y="5" width="18" height="14" rx="2" />
      <circle cx="9" cy="10" r="1.6" />
      <path d="M21 16l-5-5-8 8" />
    </>
  ),
  news: (
    <>
      <path d="M4 5h13v14H6a2 2 0 0 1-2-2z" />
      <path d="M17 9h3v8a2 2 0 0 1-2 2" />
      <path d="M8 9h5M8 13h5" />
    </>
  ),
  settings: (
    <>
      <path d="M4 7h10M18 7h2M4 17h4M12 17h8" />
      <circle cx="16" cy="7" r="2" />
      <circle cx="10" cy="17" r="2" />
    </>
  ),
  user: (
    <>
      <circle cx="12" cy="8" r="4" />
      <path d="M4 21c1.5-4 4.5-6 8-6s6.5 2 8 6" />
    </>
  ),
  plus: <path d="M12 5v14M5 12h14" />,
  play: <path d="M7 4l13 8-13 8z" fill="currentColor" stroke="none" />,
  chevron: <path d="M6 9l6 6 6-6" />,
  folder: <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />,
  edit: <path d="M4 20h4L19 9l-4-4L4 16zM14 6l4 4" />,
  trash: <path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13" />,
  image: (
    <>
      <rect x="3" y="5" width="18" height="14" rx="2" />
      <path d="M21 16l-5-5-8 8" />
    </>
  ),
  close: <path d="M6 6l12 12M18 6L6 18" />,
  stop: <rect x="6" y="6" width="12" height="12" rx="1" fill="currentColor" stroke="none" />,
  link: <path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1" />,
  copy: (
    <>
      <rect x="8" y="8" width="12" height="12" rx="2" />
      <path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2" />
    </>
  ),
  bedrock: (
    <>
      <rect x="4" y="4" width="16" height="16" rx="2" />
      <path d="M4 10h16M10 10v10" />
    </>
  ),
  check: <path d="M5 12l5 5L20 7" />,
  globe: (
    <>
      <circle cx="12" cy="12" r="9" />
      <path d="M3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18" />
    </>
  ),
  download: <path d="M12 4v11M7 10l5 5 5-5M5 20h14" />,
  upload: <path d="M12 20V9M7 14l5-5 5 5M5 4h14" />,
  pencil: <path d="M4 20h4L19 9l-4-4L4 16z" />,
  eraser: <path d="M8 20h12M5 15l8-8 6 6-6 6H9z" />,
  bucket: <path d="M5 11l7-7 7 7-7 7zM19 15c0 2 1 3 1 4a1 1 0 0 1-2 0c0-1 1-2 1-4z" />,
  picker: <path d="M14 4l6 6-2 2-6-6zM12 8l-8 8v4h4l8-8" />,
  undo: <path d="M9 7L4 12l5 5M4 12h11a5 5 0 0 1 0 10h-2" />,
  redo: <path d="M15 7l5 5-5 5M20 12H9a5 5 0 0 0 0 10h2" />,
  grid: <path d="M4 4h16v16H4zM4 10h16M4 16h16M10 4v16M16 4v16" />,
  line: <path d="M5 19L19 5" />,
  square: <rect x="5" y="5" width="14" height="14" rx="1" />,
  swap: <path d="M7 7h11l-3-3M17 17H6l3 3" />,
  sun: (
    <>
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2M12 20v2M2 12h2M20 12h2M5 5l1.5 1.5M17.5 17.5L19 19M5 19l1.5-1.5M17.5 6.5L19 5" />
    </>
  ),
  sparkle: <path d="M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8zM19 16l.8 2.2L22 19l-2.2.8L19 22l-.8-2.2L16 19l2.2-.8z" />,
  mirror: <path d="M12 3v18M9 7L4 12l5 5zM15 7l5 5-5 5z" />,
  star: <path d="M12 3l2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z" />,
  more: (
    <>
      <circle cx="5" cy="12" r="1.4" />
      <circle cx="12" cy="12" r="1.4" />
      <circle cx="19" cy="12" r="1.4" />
    </>
  ),
  terminal: <path d="M4 5h16v14H4zM7 9l3 3-3 3M12 15h5" />,
  rocket: <path d="M12 15l-3-3c1-4 4-8 10-9-1 6-5 9-9 10zM9 12l-4 1 2-4M12 15l-1 4 4-2M6 18c-1 1-1 3-1 3s2 0 3-1" />,
  copy2: <path d="M8 8h12v12H8zM4 16V4h12" />,
  palette: (
    <>
      <path d="M12 3a9 9 0 1 0 0 18c1 0 1.5-.8 1.2-1.6-.4-1 .3-2 1.4-2H17a4 4 0 0 0 4-4c0-5.5-4-10.4-9-10.4z" />
      <circle cx="7.5" cy="11" r="1.2" />
      <circle cx="11" cy="7.5" r="1.2" />
      <circle cx="15.5" cy="8.5" r="1.2" />
    </>
  ),
};

export function Icon({ name, size = 20 }: { name: keyof typeof paths | string; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {paths[name] ?? null}
    </svg>
  );
}
