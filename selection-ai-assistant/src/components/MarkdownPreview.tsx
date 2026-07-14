import type { ReactNode } from 'react';

const IMAGE_PATTERN = /!\[([^\]]*)\]\(([^)\s]+)\)/g;
const LINK_PATTERN = /\[([^\]]+)\]\(([^)\s]+)\)/g;
const STRONG_PATTERN = /(\*\*|__)(.+?)\1/g;
const TABLE_SEPARATOR_CELL_PATTERN = /^:?-{3,}:?$/;

function isSafeHttpUrl(url: string) {
  try {
    const parsed = new URL(url);
    return parsed.protocol === 'http:' || parsed.protocol === 'https:';
  } catch {
    return false;
  }
}

function isAllowedEmbeddedImageUrl(url: string) {
  return /^data:image\/(png|jpeg|jpg|gif|webp);base64,[a-z0-9+/=]+$/i.test(url);
}

function renderInlineText(text: string, keyPrefix: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  let index = 0;
  let nodeIndex = 0;

  for (const match of text.matchAll(STRONG_PATTERN)) {
    const matchIndex = match.index ?? 0;
    if (matchIndex < index) continue;
    if (matchIndex > index) nodes.push(text.slice(index, matchIndex));

    const [raw, , content] = match;
    if (content.trim()) {
      nodes.push(<strong key={`${keyPrefix}-strong-${nodeIndex}`}>{content}</strong>);
    } else {
      nodes.push(raw);
    }
    index = matchIndex + raw.length;
    nodeIndex += 1;
  }

  if (index < text.length) nodes.push(text.slice(index));
  return nodes.length > 0 ? nodes : [text];
}

function renderInlineMarkdown(text: string, keyPrefix: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  let index = 0;
  let nodeIndex = 0;
  const imageMatches = Array.from(text.matchAll(IMAGE_PATTERN)).map((match) => ({
    type: 'image' as const,
    match,
    index: match.index ?? 0,
  }));
  const linkMatches = Array.from(text.matchAll(LINK_PATTERN))
    .filter((item) => !text.slice(Math.max(0, item.index ?? 0) - 1, item.index ?? 0).endsWith('!'))
    .map((match) => ({ type: 'link' as const, match, index: match.index ?? 0 }));
  const matches = [...imageMatches, ...linkMatches].sort((a, b) => a.index - b.index);

  for (const item of matches) {
    if (item.index < index) continue;
    if (item.index > index) {
      nodes.push(...renderInlineText(text.slice(index, item.index), `${keyPrefix}-text-${nodeIndex}`));
    }

    const [raw, label, url] = item.match;
    const trimmedUrl = url.trim();
    if (item.type === 'image') {
      if (isAllowedEmbeddedImageUrl(trimmedUrl)) {
        nodes.push(<img key={`${keyPrefix}-inline-${nodeIndex}`} src={trimmedUrl} alt={label} loading="lazy" />);
      } else {
        nodes.push(
          <span key={`${keyPrefix}-inline-${nodeIndex}`} className="markdown-hidden-image">
            远程图片已阻止{label ? `：${label}` : ''}
          </span>,
        );
      }
    } else if (isSafeHttpUrl(trimmedUrl)) {
      nodes.push(
        <a key={`${keyPrefix}-inline-${nodeIndex}`} href={trimmedUrl} target="_blank" rel="noopener noreferrer">
          {renderInlineText(label, `${keyPrefix}-link-${nodeIndex}`)}
        </a>,
      );
    } else {
      nodes.push(raw);
    }

    index = item.index + raw.length;
    nodeIndex += 1;
  }

  if (index < text.length) {
    nodes.push(...renderInlineText(text.slice(index), `${keyPrefix}-text-${nodeIndex}`));
  }

  return nodes.length > 0 ? nodes : renderInlineText(text, `${keyPrefix}-text`);
}

function splitTableRow(line: string) {
  return line
    .trim()
    .replace(/^\|/, '')
    .replace(/\|$/, '')
    .split('|')
    .map((cell) => cell.trim());
}

function isTableSeparator(line: string) {
  const cells = splitTableRow(line);
  return cells.length > 1 && cells.every((cell) => TABLE_SEPARATOR_CELL_PATTERN.test(cell));
}

function isPotentialTableRow(line: string) {
  return line.includes('|') && splitTableRow(line).length > 1;
}

function renderTable(headers: string[], rows: string[][], key: string) {
  return (
    <div key={key} className="markdown-table-wrap">
      <table>
        <thead>
          <tr>
            {headers.map((header, index) => (
              <th key={index}>{renderInlineMarkdown(header, `${key}-th-${index}`)}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, rowIndex) => (
            <tr key={rowIndex}>
              {headers.map((_, cellIndex) => (
                <td key={cellIndex}>{renderInlineMarkdown(row[cellIndex] ?? '', `${key}-td-${rowIndex}-${cellIndex}`)}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function MarkdownPreview({ markdown }: { markdown: string }) {
  const lines = markdown.split(/\r?\n/);
  const blocks: ReactNode[] = [];
  let paragraph: string[] = [];
  let listItems: string[] = [];
  let codeLines: string[] = [];
  let inCodeBlock = false;

  function flushParagraph() {
    if (paragraph.length === 0) return;
    const text = paragraph.join(' ').trim();
    if (text) {
      blocks.push(<p key={`p-${blocks.length}`}>{renderInlineMarkdown(text, `p-${blocks.length}`)}</p>);
    }
    paragraph = [];
  }

  function flushList() {
    if (listItems.length === 0) return;
    blocks.push(
      <ul key={`ul-${blocks.length}`}>
        {listItems.map((item, index) => (
          <li key={index}>{renderInlineMarkdown(item, `li-${blocks.length}-${index}`)}</li>
        ))}
      </ul>,
    );
    listItems = [];
  }

  function flushCode() {
    blocks.push(
      <pre key={`pre-${blocks.length}`}>
        <code>{codeLines.join('\n')}</code>
      </pre>,
    );
    codeLines = [];
  }

  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex];

    if (line.trim().startsWith('```')) {
      flushParagraph();
      flushList();
      if (inCodeBlock) {
        flushCode();
        inCodeBlock = false;
      } else {
        inCodeBlock = true;
      }
      continue;
    }

    if (inCodeBlock) {
      codeLines.push(line);
      continue;
    }

    const trimmed = line.trim();
    if (!trimmed) {
      flushParagraph();
      flushList();
      continue;
    }

    const nextLine = lines[lineIndex + 1];
    if (isPotentialTableRow(trimmed) && nextLine && isTableSeparator(nextLine)) {
      flushParagraph();
      flushList();
      const headers = splitTableRow(trimmed);
      const rows: string[][] = [];
      lineIndex += 2;
      while (lineIndex < lines.length && isPotentialTableRow(lines[lineIndex].trim())) {
        rows.push(splitTableRow(lines[lineIndex]));
        lineIndex += 1;
      }
      lineIndex -= 1;
      blocks.push(renderTable(headers, rows, `table-${blocks.length}`));
      continue;
    }

    if (trimmed === '---') {
      flushParagraph();
      flushList();
      blocks.push(<hr key={`hr-${blocks.length}`} />);
      continue;
    }

    const heading = trimmed.match(/^(#{1,3})\s+(.+)$/);
    if (heading) {
      flushParagraph();
      flushList();
      const level = heading[1].length;
      const content = renderInlineMarkdown(heading[2].trim(), `h-${blocks.length}`);
      if (level === 1) blocks.push(<h1 key={`h-${blocks.length}`}>{content}</h1>);
      else if (level === 2) blocks.push(<h2 key={`h-${blocks.length}`}>{content}</h2>);
      else blocks.push(<h3 key={`h-${blocks.length}`}>{content}</h3>);
      continue;
    }

    const listItem = trimmed.match(/^[-*]\s+(.+)$/);
    if (listItem) {
      flushParagraph();
      listItems.push(listItem[1].trim());
      continue;
    }

    flushList();
    paragraph.push(trimmed);
  }

  if (inCodeBlock) flushCode();
  flushParagraph();
  flushList();

  return <div className="markdown-preview">{blocks.length > 0 ? blocks : null}</div>;
}
