export function summarize(value) {
  const values = [
    value,
    value + 1,
    value + 2,
  ];
  return values.join(",");
}

export function neverCalled(value) {
  const scaled = value * 2;
  return scaled - 1;
}
