type Profile = {
  name: string;
  email?: string;
  tags: string[];
};

export function buildTagSummary(tags: string[]) {
  if (tags.length === 0) {
    return "no tags";
  }

  return tags.join(" / ");
}

export function ProfileCard(props: { profile: Profile; showEmail: boolean }) {
  const normalizedTags = props.profile.tags
    .map((tag) => tag.trim())
    .filter(Boolean);
  const summary = buildTagSummary(normalizedTags);
  const details = (
    <dl>
      <div>
        <dt>Name</dt>
        <dd>{props.profile.name}</dd>
      </div>
      <div>
        <dt>Tags</dt>
        <dd>{summary}</dd>
      </div>
    </dl>
  );

  return (
    <section data-kind="profile">
      <header>
        <h2>{props.profile.name}</h2>
      </header>
      {details}
      {props.showEmail ? (
        <span>{props.profile.email ?? "unknown"}</span>
      ) : (
        <span>hidden</span>
      )}
    </section>
  );
}

export function CompactProfile(props: { profile: Profile }) {
  return (
    <aside>
      <strong>{props.profile.name}</strong>
      <small>{buildTagSummary(props.profile.tags)}</small>
    </aside>
  );
}
