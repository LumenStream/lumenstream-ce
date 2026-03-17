interface Props {
  title: string;
  description: string;
  href?: string;
  linkLabel?: string;
}

export function FeatureUnavailable({ title, description, href, linkLabel }: Props) {
  return (
    <section className="border-border bg-card/60 space-y-3 rounded-2xl border p-6">
      <p className="text-muted-foreground text-xs font-semibold tracking-[0.16em] uppercase">
        Community Edition
      </p>
      <div className="space-y-2">
        <h2 className="text-foreground text-xl font-semibold">{title}</h2>
        <p className="text-muted-foreground text-sm leading-6">{description}</p>
      </div>
      {href && linkLabel ? (
        <a
          href={href}
          className="text-sm font-medium text-rose-300 transition-colors hover:text-rose-200"
        >
          {linkLabel}
        </a>
      ) : null}
    </section>
  );
}
