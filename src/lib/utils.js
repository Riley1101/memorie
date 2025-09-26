import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";

/**
 * @param  {...any} inputs string array of classes
 * @returns combined string of tailwindcss classes
 */
export function cn(...inputs) {
  return twMerge(clsx(inputs));
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
