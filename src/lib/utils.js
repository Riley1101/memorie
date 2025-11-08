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
