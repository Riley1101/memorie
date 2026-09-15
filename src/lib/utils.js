import { clsx } from 'clsx';
import DOMPurify from 'dompurify';
import { twMerge } from 'tailwind-merge';

/**
 * @param  {...any} inputs string array of classes
 * @returns combined string of tailwindcss classes
 */
export function cn(...inputs) {
  return twMerge(clsx(inputs));
}

/**
 * Trim and uppercase a file with .md extension
 *
 * @param {string} name - file name
 * @returns string
 */
export function formatFileName(name) {
  const ext = name.split('.').pop();
  if (ext === 'md') {
    return name.replace('.md', '').trim();
  }
  return name.trim();
}

/**
 * Sanitize Markdown text before rendering.
 * - Removes malicious or unwanted HTML.
 * - Normalizes line breaks and whitespace.
 * @param {string} md - Markdown text
 */
export function sanitizeMarkdown(md) {
  if (!md) return '';

  let sanitized = md;

  sanitized = sanitized
    .replace(/\r\n/g, '\n')
    .replace(/\n{3,}/g, '\n\n')
    .replace(/[ \t]+$/gm, '')
    .trim();

  sanitized = DOMPurify.sanitize(sanitized);

  sanitized = sanitized
    .replace(/^```[\w-]*\n([\s\S]*?)\n```$/gm, '```$1```')
    .replace(/<!--.*?-->/gs, '')
    .replace(/<\/?[^>]+(>|$)/g, (match) => match);

  return sanitized;
}

/**
 * Builds a nested folder/file tree for the files inside one binder, from the
 * flat `FileEntry[]` list the backend returns (each entry's `name` carries its
 * full path, e.g. "Novel/Chapter 1/Scene 1.md"). Used to render chapters/scenes
 * as a real tree instead of a flat list. Folders sort before files, both
 * alphabetically.
 *
 * @param {Array<{name: string, folder: string|null}>} files
 * @param {string} binder
 * @param {string[]} [folders] - `/`-joined relative folder paths (e.g. "Novel/Chapter 2"),
 *   from `fileManager.folders`. Passed so folders with no writings in them yet still show up.
 * @returns {Array<{type: 'folder', name: string, path: string[], children: Array} | {type: 'file', file: object}>}
 */
/** "Chapter 2" before "Chapter 10", case-insensitive. */
export const naturalCollator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });

export function buildFileTree(files, binder, folders = []) {
  const root = [];

  const getOrCreateFolder = (level, path, folderName) => {
    let node = level.find((n) => n.type === 'folder' && n.name === folderName);
    if (!node) {
      node = { type: 'folder', name: folderName, path, children: [], fileCount: 0, lastModified: 0 };
      level.push(node);
    }
    return node;
  };

  for (const folderPath of folders) {
    const segments = folderPath.split('/');
    if (segments[0] !== binder || segments.length < 2) continue;

    let level = root;
    const path = [];
    for (const folderName of segments.slice(1)) {
      path.push(folderName);
      level = getOrCreateFolder(level, [...path], folderName).children;
    }
  }

  for (const file of files) {
    if (file.folder !== binder) continue;

    const rest = file.name.slice(binder.length + 1).split('/');
    const folderNames = rest.slice(0, -1);
    const fileName = rest[rest.length - 1];

    let level = root;
    const path = [];
    for (const folderName of folderNames) {
      path.push(folderName);
      level = getOrCreateFolder(level, [...path], folderName).children;
    }

    level.push({ type: 'file', name: fileName, file });
  }

  // Sort, and roll file counts / last-edited up through each folder.
  const finish = (nodes) => {
    nodes.sort((a, b) => {
      if (a.type !== b.type) return a.type === 'folder' ? -1 : 1;
      return naturalCollator.compare(a.name, b.name);
    });
    let count = 0;
    let latest = 0;
    for (const node of nodes) {
      if (node.type === 'folder') {
        const sub = finish(node.children);
        node.fileCount = sub.count;
        node.lastModified = sub.latest;
        count += sub.count;
        latest = Math.max(latest, sub.latest);
      } else {
        count += 1;
        latest = Math.max(latest, node.file.last_modified ?? 0);
      }
    }
    return { count, latest };
  };
  finish(root);

  return root;
}

/**
 * Formats a date as a relative time string (e.g., "5 minutes ago").
 * @param date - Date object
 * @returns {string}
 */
export function formatTimeAgo(date) {
  if (!date) return '';
  
  const seconds = Math.floor((new Date() - date) / 1000);

  if (seconds < 5) {
    return 'just now';
  } else if (seconds < 60) {
    return `${seconds} seconds ago`;
  } else if (seconds < 3600) {
    // Less than an hour
    const minutes = Math.floor(seconds / 60);
    return `${minutes} minute${minutes > 1 ? 's' : ''} ago`;
  } else if (seconds < 86400) {
    // Less than a day
    const hours = Math.floor(seconds / 3600);
    return `${hours} hour${hours > 1 ? 's' : ''} ago`;
  } else if (seconds < 2592000) {
    // Less than a month (approx. 30 days)
    const days = Math.floor(seconds / 86400);
    return `${days} day${days > 1 ? 's' : ''} ago`;
  } else if (seconds < 31536000) {
    // Less than a year (approx. 365 days)
    const months = Math.floor(seconds / 2592000);
    return `${months} month${months > 1 ? 's' : ''} ago`;
  } else {
    const years = Math.floor(seconds / 31536000);
    return `${years} year${years > 1 ? 's' : ''} ago`;
  }
}

/**
 * Formats a unix timestamp (seconds) as a short date, e.g. "Jan 5".
 * @param {number} timestamp
 * @returns {string}
 */
export function formatDate(timestamp) {
  return new Date(timestamp * 1000).toLocaleDateString([], { month: 'short', day: 'numeric' });
}

/**
 * Formats a unix timestamp (seconds) as a short time, e.g. "3:45 PM".
 * @param {number} timestamp
 * @returns {string}
 */
export function formatTime(timestamp) {
  return new Date(timestamp * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}
