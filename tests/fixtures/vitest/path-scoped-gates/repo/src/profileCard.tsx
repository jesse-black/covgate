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
  const summary = buildTagSummary(props.profile.tags);

  return (
    <section data-kind="profile">
      <h2>{props.profile.name}</h2>
      <p>{summary}</p>
      {props.showEmail ? (
        <span>{props.profile.email ?? "unknown"}</span>
      ) : (
        <span>hidden</span>
      )}
    </section>
  );
}
