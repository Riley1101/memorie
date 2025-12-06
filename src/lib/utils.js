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
 * Formats a date as a relative time string (e.g., "5 minutes ago").
 * @param date - Date object
 * @returns {string}
 */
export function formatTimeAgo(date) {
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
