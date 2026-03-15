export function adjust(value) {
  if (value >= 0) {
    return value + 1;
  }

  return value - 1;
}

export function neverCalled(value) {
  if (value === 0) {
    return 0;
  }

  return value * 2;
}
