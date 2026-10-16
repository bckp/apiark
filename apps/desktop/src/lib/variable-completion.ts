/** Shared by native inputs and Monaco; completion objects contain names only. */
export function variableCompletion(
  text: string,
  cursor: number,
  names: readonly string[],
) {
  const match = text.slice(0, cursor).match(/\{\{([^{}\n]*)$/);
  if (!match) return null;
  const start = cursor - match[0].length;
  const tail =
    text.slice(cursor).match(/^[^{}\s]*\}\}/)?.[0] ??
    text.slice(cursor).match(/^[\p{L}\p{N}_.$-]*/u)?.[0] ??
    "";
  const end = cursor + tail.length;
  const query = match[1].trim().toLowerCase();
  const candidates = [...new Set(names)]
    .filter(
      (name) =>
        name.length > 0 &&
        !/[{}\r\n]/.test(name) &&
        name.toLowerCase().includes(query),
    )
    .sort((a, b) => a.localeCompare(b));
  return { start, end, candidates };
}

export function insertVariable(
  text: string,
  range: { start: number; end: number },
  name: string,
) {
  const token = `{{${name}}}`;
  return {
    value: text.slice(0, range.start) + token + text.slice(range.end),
    cursor: range.start + token.length,
  };
}
