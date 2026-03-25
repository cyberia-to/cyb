export function shortenString(string: string, length = 300) {
  return string.length > length ? `${string.slice(0, length)}...` : string;
}

export function replaceQuotes(string: string) {
  return string.replace(/"/g, "'");
}

export function serializeString(input: string): string {
  return input
    .replace(/\\/g, '\\\\') // Escape backslashes
    .replace(/"/g, "\\''") // Escape double quotes
    .replace(/'/g, "\\'") // Escape single quotes
    .replace(/\n/g, '\\n') // Escape newlines
    .replace(/\r/g, '\\r') // Escape carriage returns
    .replace(/#/g, '\\!!'); // Escape  - that's comment in cozo
}

export function deserializeString(serialized: string): string {
  return serialized
    .replace(/\\r/g, '\r') // Unescape carriage returns
    .replace(/\\n/g, '\n') // Unescape newlines
    .replace(/\\'/g, "'") // Unescape single quotes
    .replace(/\\''/g, '"') // Unescape double quotes
    .replace(/\\\\/g, '\\') // Unescape backslashes
    .replace(/\\!!/g, '#'); // Unescape # cozo comment
}

const specialCharsRegexe = /\\u\{[a-fA-F0-9]+\}/g;

export function removeBrokenUnicode(string: string): string {
  return string.replace(specialCharsRegexe, '');
}

export function removeMarkdownFormatting(markdown: string): string {
  // Remove headers
  let text = markdown.replace(/^#{1,6}\s+/gm, '');

  // Remove emphasis (bold, italic, strikethrough)
  text = text.replace(/(\*\*|__)(.*?)\1/g, '$2');
  text = text.replace(/(\*|_)(.*?)\1/g, '$2');
  text = text.replace(/(~~)(.*?)\1/g, '$2');

  // Remove inline code and code blocks
  text = text.replace(/`{1,3}[^`](.*?)`{1,3}/g, '$1');
  text = text.replace(/```[\s\S]*?```/g, '');

  // Remove blockquotes
  text = text.replace(/^\s{0,3}>\s?/gm, '');

  // Remove links
  text = text.replace(/\[(.*?)\]\(.*?\)/g, '$1');

  // Remove images
  text = text.replace(/!\[(.*?)\]\(.*?\)/g, '$1');

  // Remove horizontal rules
  text = text.replace(/^-{3,}$/gm, '');

  // Remove unordered lists
  text = text.replace(/^\s*[-+*]\s+/gm, '');

  // Remove ordered lists
  text = text.replace(/^\s*\d+\.\s+/gm, '');

  // Remove extra spaces and new lines
  text = text.replace(/\n{2,}/g, '\n\n');
  text = text.replace(/^\s+|\s+$/g, '');

  return text;
}
