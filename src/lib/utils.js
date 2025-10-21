import { clsx } from 'clsx';
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
