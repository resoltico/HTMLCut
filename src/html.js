import { basename } from 'node:path';
import { parse, parseFragment, serialize, serializeOuter } from 'parse5';

const SKIP_TAGS = new Set(['script', 'style', 'noscript', 'template', 'head']);
const BLOCK_TAGS = new Set([
    'address',
    'article',
    'aside',
    'blockquote',
    'details',
    'dialog',
    'div',
    'dl',
    'dt',
    'dd',
    'figure',
    'figcaption',
    'footer',
    'form',
    'h1',
    'h2',
    'h3',
    'h4',
    'h5',
    'h6',
    'header',
    'hr',
    'li',
    'main',
    'nav',
    'ol',
    'p',
    'pre',
    'section',
    'summary',
    'table',
    'tbody',
    'thead',
    'tfoot',
    'tr',
    'ul',
]);
const URL_ATTRIBUTES = new Set(['href', 'src', 'poster', 'action', 'formaction', 'cite', 'data']);
const ORDERED_LIST_TYPES = new Set(['1', 'a', 'A', 'i', 'I']);
const DEFAULT_ORDERED_LIST_TYPE = '1';
const MIN_ROMAN = 1;
const MAX_ROMAN = 3999;
const ASCII_UPPERCASE_A = 65;
const ASCII_LOWERCASE_A = 97;
const LATIN_ALPHABET_LENGTH = 26;
const TABLE_MIN_COLUMN_WIDTH = 3;
const PERCENT_SCALE = 100;
const SETEXT_HEADING_LEVELS = new Map([
    [1, '='],
    [2, '-'],
]);
/* eslint-disable no-magic-numbers */
const ROMAN_NUMERAL_STEPS = [
    ['M', 1000],
    ['CM', 900],
    ['D', 500],
    ['CD', 400],
    ['C', 100],
    ['XC', 90],
    ['L', 50],
    ['XL', 40],
    ['X', 10],
    ['IX', 9],
    ['V', 5],
    ['IV', 4],
    ['I', 1],
];
/* eslint-enable no-magic-numbers */

function isHtmlDocument(fragment) {
    return /^\s*(?:<!doctype\b[^>]*>\s*)?<html\b/i.test(fragment) || /^\s*<!doctype\b/i.test(fragment);
}

function detectSpecialRootFragment(fragment) {
    const match = /^\s*<(html|head|body)\b[^>]*>/i.exec(fragment);
    if (!match) {
        return '';
    }

    const tagName = match[1].toLowerCase();
    return new RegExp(`<\\/${tagName}>\\s*$`, 'i').test(fragment) ? tagName : '';
}

function isElement(node, tagName = '') {
    return Boolean(node?.tagName) && (tagName === '' || node.tagName === tagName);
}

function isText(node) {
    return node?.nodeName === '#text';
}

function isComment(node) {
    return node?.nodeName === '#comment';
}

function getChildren(node) {
    return Array.isArray(node?.childNodes) ? node.childNodes : [];
}

function getAttribute(node, name) {
    return node.attrs?.find(attr => attr.name === name)?.value ?? null;
}

function hasAttribute(node, name) {
    return node.attrs?.some(attr => attr.name === name) ?? false;
}

function parseIntegerAttribute(node, name) {
    const value = getAttribute(node, name);
    if (value === null || !/^-?\d+$/.test(value.trim())) {
        return null;
    }

    return Number.parseInt(value, 10);
}

function normalizeControlText(text) {
    return text.replace(/\u00a0/g, ' ').replace(/\r\n?/g, '\n').trim();
}

function parseHtmlTree(html) {
    if (isHtmlDocument(html)) {
        const document = parse(html);
        const htmlElement = getDocumentChild(document, 'html');
        return {
            kind: 'document',
            root: document,
            document,
            html: htmlElement,
            preserveNode: document,
        };
    }

    const specialRoot = detectSpecialRootFragment(html);
    if (specialRoot) {
        const document = parse(specialRoot === 'html'
            ? `<!DOCTYPE html>${html}`
            : specialRoot === 'head'
                ? `<!DOCTYPE html><html>${html}<body></body></html>`
                : `<!DOCTYPE html><html><head></head>${html}</html>`);
        const htmlElement = getDocumentChild(document, 'html');
        const preserveNode = specialRoot === 'html'
            ? htmlElement
            : getChildren(htmlElement).find(child => isElement(child, specialRoot)) ?? null;

        return {
            kind: 'special-root',
            root: document,
            document,
            html: htmlElement,
            preserveNode,
        };
    }

    return {
        kind: 'fragment',
        root: parseFragment(html),
        document: null,
        html: null,
        preserveNode: null,
    };
}

function getDocumentChild(document, tagName) {
    return getChildren(document).find(child => isElement(child, tagName)) ?? null;
}

function getDocumentSection(documentTree, tagName) {
    return documentTree.html ? getChildren(documentTree.html).find(child => isElement(child, tagName)) ?? null : null;
}

function isSkippable(node) {
    return isComment(node) || (isElement(node) && SKIP_TAGS.has(node.tagName));
}

function isBlockNode(node) {
    return isElement(node) && BLOCK_TAGS.has(node.tagName);
}

const blockDescendantCache = new WeakMap();

function hasBlockDescendant(node) {
    if (!isElement(node)) {
        return false;
    }

    if (blockDescendantCache.has(node)) {
        return blockDescendantCache.get(node);
    }

    const value = getChildren(node).some(child => !isSkippable(child) && (isBlockNode(child) || hasBlockDescendant(child)));
    blockDescendantCache.set(node, value);
    return value;
}

function shouldRenderAsBlock(node) {
    return isElement(node) && (isBlockNode(node) || hasBlockDescendant(node));
}

function joinBlocks(blocks) {
    return blocks.filter(Boolean).join('\n\n');
}

function prefixLines(text, prefix) {
    const emptyLinePrefix = prefix.endsWith(' ') ? prefix.slice(0, -1) : prefix;
    return text
        .split('\n')
        .map(line => line ? `${prefix}${line}` : emptyLinePrefix)
        .join('\n');
}

function normalizeInlineText(text) {
    return text
        .replace(/\u00a0/g, ' ')
        .replace(/\r\n?/g, '\n')
        .split('\n')
        .map(line => line.replace(/[ \t\f\v]+/g, ' ').trim())
        .join('\n')
        .replace(/\n{3,}/g, '\n\n')
        .trim();
}

function renderHeading(node, ctx) {
    const headingText = renderInlineSequence(getChildren(node), ctx).replace(/\s*\n\s*/g, ' ').trim();
    if (!headingText) {
        return '';
    }

    const level = Number(node.tagName[1]);
    const underlineCharacter = SETEXT_HEADING_LEVELS.get(level);
    if (underlineCharacter) {
        return `${headingText}\n${underlineCharacter.repeat(headingText.length)}`;
    }

    return `${'#'.repeat(level)} ${headingText}`;
}

function collectRawText(node) {
    if (isSkippable(node)) {
        return '';
    }

    if (isText(node)) {
        return node.value;
    }

    return getChildren(node).map(child => collectRawText(child)).join('');
}

function renderInlineSequence(nodes, ctx) {
    return normalizeInlineText(joinInlineParts(nodes.map(node => renderInlineNode(node, ctx))));
}

function shouldInsertInlineGap(left, right) {
    if (!left || !right || /\s$/.test(left) || /^\s/.test(right)) {
        return false;
    }

    return /[.:;!?)]$/.test(left) && /^[\p{L}\p{N}"'(]/u.test(right);
}

function joinInlineParts(parts) {
    let text = '';

    for (const part of parts) {
        if (!part) {
            continue;
        }

        if (shouldInsertInlineGap(text, part)) {
            text += ' ';
        }

        text += part;
    }

    return text;
}

function renderSelectedOptions(node, ctx) {
    const options = getChildren(node).filter(child => isElement(child, 'option'));
    if (options.length === 0) {
        return '';
    }

    const selected = options.filter(option => hasAttribute(option, 'selected'));
    const active = selected.length > 0
        ? selected
        : hasAttribute(node, 'multiple')
            ? []
            : [options[0]];
    const text = active
        .map(option => renderInlineSequence(getChildren(option), ctx))
        .filter(Boolean)
        .join(', ');

    return text ? ` ${text}` : '';
}

function renderTextarea(node) {
    const text = normalizeControlText(collectRawText(node));
    return text ? `\n${text}` : '';
}

function renderInput(node) {
    const type = (getAttribute(node, 'type') || 'text').toLowerCase();
    const value = getAttribute(node, 'value');
    const placeholder = getAttribute(node, 'placeholder');

    switch (type) {
    case 'hidden':
        return '';
    case 'checkbox':
        return hasAttribute(node, 'checked') ? ' [x]' : ' [ ]';
    case 'radio':
        return hasAttribute(node, 'checked') ? ' (o)' : ' ( )';
    case 'password':
        return value ? ' [password]' : '';
    case 'button':
    case 'submit':
    case 'reset':
        return value ? ` ${value}` : '';
    case 'image':
        return normalizeControlText(getAttribute(node, 'alt') ?? '') ? ` ${normalizeControlText(getAttribute(node, 'alt'))}` : '';
    case 'file':
        return value ? ` ${value}` : ' [file]';
    default: {
        const text = normalizeControlText(value ?? placeholder ?? '');
        return text ? ` ${text}` : '';
    }
    }
}

function renderProgressLike(node, ctx) {
    const fallbackText = renderInlineSequence(getChildren(node), ctx);
    if (fallbackText) {
        return fallbackText;
    }

    const value = Number(getAttribute(node, 'value'));
    const max = Number(getAttribute(node, 'max') ?? '1');
    if (!Number.isFinite(value) || !Number.isFinite(max) || max <= 0) {
        return '';
    }

    const percent = Math.round((value / max) * PERCENT_SCALE);
    return `${percent}%`;
}

function renderInlineNode(node, ctx) {
    if (isSkippable(node)) {
        return '';
    }

    if (isText(node)) {
        return node.value;
    }

    if (!isElement(node)) {
        return '';
    }

    switch (node.tagName) {
    case 'rt':
    case 'rp':
        return '';

    case 'br':
        return '\n';

    case 'img': {
        const alt = normalizeInlineText(getAttribute(node, 'alt') ?? '');
        return alt ? `[${alt}]` : '';
    }

    case 'code':
        if (ctx.inPre) {
            return collectRawText(node);
        }
        return (() => {
            const codeText = renderInlineSequence(getChildren(node), ctx);
            return codeText ? `\`${codeText}\`` : '';
        })();

    case 'a': {
        const text = renderInlineSequence(getChildren(node), ctx);
        const href = getAttribute(node, 'href');

        if (!href || href.startsWith('#')) {
            return text;
        }

        if (!text) {
            return `[${href}]`;
        }

        if (text === href) {
            return text;
        }

        return `${text} [${href}]`;
    }

    case 'input':
        return renderInput(node);

    case 'textarea':
        return renderTextarea(node);

    case 'select':
        return renderSelectedOptions(node, ctx);

    case 'progress':
    case 'meter':
        return renderProgressLike(node, ctx);

    default:
        return getChildren(node).map(child => renderInlineNode(child, ctx)).join('');
    }
}

function normalizeOrderedListType(value) {
    if (!value) {
        return DEFAULT_ORDERED_LIST_TYPE;
    }

    return ORDERED_LIST_TYPES.has(value) ? value : DEFAULT_ORDERED_LIST_TYPE;
}

function toAlphabeticCounter(value, uppercase) {
    if (value <= 0) {
        return String(value);
    }

    const chars = [];
    let remainder = value;

    while (remainder > 0) {
        remainder--;
        chars.unshift(String.fromCharCode((uppercase ? ASCII_UPPERCASE_A : ASCII_LOWERCASE_A) + (remainder % LATIN_ALPHABET_LENGTH)));
        remainder = Math.floor(remainder / LATIN_ALPHABET_LENGTH);
    }

    return chars.join('');
}

function toRomanCounter(value, uppercase) {
    if (value < MIN_ROMAN || value > MAX_ROMAN) {
        return String(value);
    }

    let remainder = value;
    let result = '';

    for (const [numeral, amount] of ROMAN_NUMERAL_STEPS) {
        while (remainder >= amount) {
            result += numeral;
            remainder -= amount;
        }
    }

    return uppercase ? result : result.toLowerCase();
}

function formatOrderedListMarker(value, type) {
    switch (type) {
    case 'a':
        return `${toAlphabeticCounter(value, false)}.`;
    case 'A':
        return `${toAlphabeticCounter(value, true)}.`;
    case 'i':
        return `${toRomanCounter(value, false)}.`;
    case 'I':
        return `${toRomanCounter(value, true)}.`;
    default:
        return `${value}.`;
    }
}

function createOrderedListState(node, itemCount) {
    const reversed = hasAttribute(node, 'reversed');
    const start = parseIntegerAttribute(node, 'start');
    return {
        counter: start ?? (reversed ? itemCount : 1),
        step: reversed ? -1 : 1,
        type: normalizeOrderedListType(getAttribute(node, 'type')),
    };
}

function getListMarker(node, orderedState) {
    if (!orderedState) {
        return '-';
    }

    const itemValue = parseIntegerAttribute(node, 'value');
    if (itemValue !== null) {
        orderedState.counter = itemValue;
    }

    const itemType = getAttribute(node, 'type');
    const marker = formatOrderedListMarker(
        orderedState.counter,
        itemType ? normalizeOrderedListType(itemType) : orderedState.type
    );

    orderedState.counter += orderedState.step;
    return marker;
}

function renderList(node, ctx, ordered) {
    const items = getChildren(node).filter(child => isElement(child, 'li'));
    const orderedState = ordered ? createOrderedListState(node, items.length) : null;
    const lines = [];

    items.forEach(item => {
        const marker = getListMarker(item, orderedState);
        lines.push(renderListItem(item, ctx, marker));
    });

    return lines.filter(Boolean).join('\n');
}

function renderListItem(node, ctx, marker) {
    const indent = '  '.repeat(ctx.listDepth);
    const contentIndent = `${indent}  `;
    const blocks = [];
    let inlineNodes = [];

    function flushInline() {
        if (inlineNodes.length === 0) {
            return;
        }

        const text = renderInlineSequence(inlineNodes, { ...ctx, inPre: false });
        if (text) {
            blocks.push(text);
        }
        inlineNodes = [];
    }

    for (const child of getChildren(node)) {
        if (isSkippable(child)) {
            continue;
        }

        if (shouldRenderAsBlock(child)) {
            flushInline();
            const block = renderBlockNode(child, { ...ctx, listDepth: ctx.listDepth + 1 });
            if (block) {
                blocks.push(block);
            }
            continue;
        }

        inlineNodes.push(child);
    }

    flushInline();

    if (blocks.length === 0) {
        return `${indent}${marker}`;
    }

    const [firstBlock, ...restBlocks] = blocks;
    const lines = firstBlock.split('\n');
    const rendered = [`${indent}${marker} ${lines[0]}`];

    for (const line of lines.slice(1)) {
        rendered.push(`${contentIndent}${line}`);
    }

    for (const block of restBlocks) {
        for (const line of block.split('\n')) {
            rendered.push(`${contentIndent}${line}`);
        }
    }

    return rendered.join('\n');
}

function renderDescriptionList(node, ctx) {
    const blocks = [];

    for (const child of getChildren(node)) {
        if (isElement(child, 'dt')) {
            const term = joinBlocks(renderFlowChildren(getChildren(child), ctx));
            if (term) {
                blocks.push(term);
            }
        } else if (isElement(child, 'dd')) {
            const definition = joinBlocks(renderFlowChildren(getChildren(child), ctx));
            if (definition) {
                blocks.push(prefixLines(definition, '  '));
            }
        }
    }

    return joinBlocks(blocks);
}

function normalizeTableCellText(node, ctx) {
    return joinBlocks(renderFlowChildren(getChildren(node), ctx)).replace(/\s*\n\s*/g, ' / ').trim();
}

function parseSpanAttribute(node, name) {
    const value = parseIntegerAttribute(node, name);
    return value && value > 0 ? value : 1;
}

function collectTableStructure(node) {
    const captions = [];
    const rows = [];

    function visitTableChildren(children, section = '') {
        for (const child of children) {
            if (isSkippable(child)) {
                continue;
            }

            if (isElement(child, 'caption')) {
                captions.push(child);
                continue;
            }

            if (isElement(child, 'thead') || isElement(child, 'tbody') || isElement(child, 'tfoot')) {
                visitTableChildren(getChildren(child), child.tagName);
                continue;
            }

            if (isElement(child, 'tr')) {
                rows.push({ node: child, section });
            }
        }
    }

    visitTableChildren(getChildren(node));
    return { captions, rows };
}

function expandTableRows(rows, ctx) {
    const spans = [];
    const expandedRows = [];
    let maxColumns = 0;

    for (const row of rows) {
        const expanded = [];
        let columnIndex = 0;

        function consumeActiveSpans() {
            while (spans[columnIndex]) {
                expanded[columnIndex] = '';
                spans[columnIndex].remainingRows--;
                if (spans[columnIndex].remainingRows === 0) {
                    spans[columnIndex] = null;
                }
                columnIndex++;
            }
        }

        consumeActiveSpans();

        const cells = getChildren(row.node)
            .filter(cell => isElement(cell, 'th') || isElement(cell, 'td'))
            .map(cell => ({
                text: normalizeTableCellText(cell, ctx),
                isHeader: cell.tagName === 'th',
                colspan: parseSpanAttribute(cell, 'colspan'),
                rowspan: parseSpanAttribute(cell, 'rowspan'),
            }));

        for (const cell of cells) {
            consumeActiveSpans();
            expanded[columnIndex] = cell.text;

            for (let offset = 0; offset < cell.colspan; offset++) {
                if (offset > 0) {
                    expanded[columnIndex + offset] = '';
                }

                if (cell.rowspan > 1) {
                    spans[columnIndex + offset] = { remainingRows: cell.rowspan - 1 };
                }
            }

            columnIndex += cell.colspan;
        }

        consumeActiveSpans();
        maxColumns = Math.max(maxColumns, expanded.length);
        expandedRows.push({
            cells: expanded,
            isHeaderRow: row.section === 'thead' || (cells.length > 0 && cells.every(cell => cell.isHeader)),
        });
    }

    for (const row of expandedRows) {
        while (row.cells.length < maxColumns) {
            row.cells.push('');
        }
    }

    return expandedRows;
}

function renderAlignedTable(rows) {
    if (rows.length === 0) {
        return '';
    }

    const widths = rows[0].cells.map((_cell, columnIndex) => Math.max(
        TABLE_MIN_COLUMN_WIDTH,
        ...rows.map(row => row.cells[columnIndex]?.length ?? 0)
    ));

    const renderRow = cells => `| ${cells.map((cell, index) => cell.padEnd(widths[index])).join(' | ')} |`;
    const headerBoundary = rows.some(row => row.isHeaderRow)
        ? rows.findLastIndex
            ? rows.findLastIndex(row => row.isHeaderRow)
            : rows.reduce((last, row, index) => row.isHeaderRow ? index : last, -1)
        : -1;
    const lines = rows.map(row => renderRow(row.cells));

    if (headerBoundary >= 0) {
        lines.splice(headerBoundary + 1, 0, `| ${widths.map(width => '-'.repeat(width)).join(' | ')} |`);
    }

    return lines.join('\n');
}

function renderTable(node, ctx) {
    const { captions, rows: tableRows } = collectTableStructure(node);
    const rows = expandTableRows(tableRows, ctx);
    const captionText = captions
        .map(caption => joinBlocks(renderFlowChildren(getChildren(caption), ctx)))
        .filter(Boolean)
        .map(text => `Table: ${text}`);
    const tableText = renderAlignedTable(rows);

    return joinBlocks([...captionText, tableText]);
}

function renderBlockNode(node, ctx) {
    switch (node.tagName) {
    case 'h1':
    case 'h2':
    case 'h3':
    case 'h4':
    case 'h5':
    case 'h6': {
        return renderHeading(node, ctx);
    }

    case 'blockquote': {
        const inner = joinBlocks(renderFlowChildren(getChildren(node), ctx));
        return inner ? prefixLines(inner, '> ') : '';
    }

    case 'pre': {
        const text = collectRawText(node).replace(/\r\n?/g, '\n').replace(/^\n+|\n+$/g, '');
        return text;
    }

    case 'ul':
        return renderList(node, ctx, false);

    case 'ol':
        return renderList(node, ctx, true);

    case 'dl':
        return renderDescriptionList(node, ctx);

    case 'table':
        return renderTable(node, ctx);

    case 'hr':
        return '---';

    default:
        return joinBlocks(renderFlowChildren(getChildren(node), ctx));
    }
}

function renderFlowChildren(nodes, ctx) {
    const blocks = [];
    let inlineNodes = [];

    function flushInline() {
        if (inlineNodes.length === 0) {
            return;
        }

        const text = renderInlineSequence(inlineNodes, ctx);
        if (text) {
            blocks.push(text);
        }

        inlineNodes = [];
    }

    for (const node of nodes) {
        if (isSkippable(node)) {
            continue;
        }

        if (shouldRenderAsBlock(node)) {
            flushInline();
            const block = renderBlockNode(node, ctx);
            if (block) {
                blocks.push(block);
            }
            continue;
        }

        inlineNodes.push(node);
    }

    flushInline();
    return blocks;
}

function visitNodes(node, visitor) {
    visitor(node);
    for (const child of getChildren(node)) {
        visitNodes(child, visitor);
    }
}

export function resolveUrl(value, baseUrl) {
    if (!value || !baseUrl || value.startsWith('#') || /^[a-z][a-z0-9+.-]*:/i.test(value)) {
        return value;
    }

    try {
        return new URL(value, baseUrl).href;
    } catch {
        return value;
    }
}

export function rewriteHtmlUrls(fragment, baseUrl) {
    if (!baseUrl) {
        return fragment;
    }

    const tree = parseHtmlTree(fragment);

    visitNodes(tree.root, node => {
        if (!isElement(node)) {
            return;
        }

        for (const attr of node.attrs) {
            if (URL_ATTRIBUTES.has(attr.name)) {
                attr.value = resolveUrl(attr.value, baseUrl);
            }
        }
    });

    if (tree.preserveNode && tree.preserveNode !== tree.root) {
        return serializeOuter(tree.preserveNode);
    }

    return serialize(tree.root);
}

export function renderHtmlAsText(fragment) {
    const tree = parseHtmlTree(fragment);
    const body = tree.kind === 'document'
        || (tree.kind === 'special-root' && tree.preserveNode?.tagName === 'html')
        ? getDocumentSection(tree, 'body')
        : null;
    const roots = body
        ? getChildren(body)
        : tree.preserveNode && tree.preserveNode !== tree.root
            ? getChildren(tree.preserveNode)
            : getChildren(tree.root);
    return joinBlocks(renderFlowChildren(roots, { inPre: false, listDepth: 0 })).trim();
}

export function extractTitleFromHtml(fragment) {
    const tree = parseHtmlTree(fragment);
    let title = '';

    if (tree.kind === 'document') {
        const head = getDocumentSection(tree, 'head');
        if (head) {
            visitNodes(head, node => {
                if (!title && isElement(node, 'title')) {
                    title = collectRawText(node).trim();
                }
            });
        }
    }

    visitNodes(tree.root, node => {
        if (!title && isElement(node, 'title')) {
            title = collectRawText(node).trim();
        }
    });

    return title;
}

function deriveSourceTitle(source) {
    if (!source || source === '-') {
        return '';
    }

    try {
        return new URL(source).hostname;
    } catch {
        const stem = basename(source).replace(/\.[^.]+$/, '');
        return stem;
    }
}

function encodeHtmlText(text) {
    return text.replace(/[&<>"]/g, char => {
        switch (char) {
        case '&':
            return '&amp;';
        case '<':
            return '&lt;';
        case '>':
            return '&gt;';
        case '"':
            return '&quot;';
        default:
            return char;
        }
    });
}

export function deriveDocumentTitle({ matches, input, baseUrl }) {
    for (const match of matches) {
        const title = extractTitleFromHtml(match.html);
        if (title) {
            return title;
        }
    }

    return deriveSourceTitle(baseUrl) || deriveSourceTitle(input) || 'HTMLCut Selection';
}

export function wrapHtmlDocument({ matches, title }) {
    if (matches.length === 1 && isHtmlDocument(matches[0].html)) {
        return matches[0].html;
    }

    const body = matches.map(match => {
        const heading = matches.length > 1 ? `\n<h2>Match ${match.index}</h2>` : '';
        return `<section data-match-index="${match.index}">${heading}\n${match.html}\n</section>`;
    }).join('\n\n');

    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>${encodeHtmlText(title)}</title>
  <style>
    :root { color-scheme: light; }
    body { font-family: Georgia, "Times New Roman", serif; margin: 2rem auto; max-width: 72rem; padding: 0 1.25rem 3rem; line-height: 1.6; }
    section + section { border-top: 1px solid #d6d6d6; margin-top: 2rem; padding-top: 2rem; }
    h2 { font-family: "Avenir Next", "Helvetica Neue", sans-serif; font-size: 1rem; letter-spacing: 0.08em; text-transform: uppercase; }
  </style>
</head>
<body>
${body}
</body>
</html>`;
}
