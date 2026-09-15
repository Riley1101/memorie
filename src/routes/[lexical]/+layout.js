import { invoke } from '@tauri-apps/api/core';

/**
 * `?new=1` marks an unsaved draft: the URL names where the file will live once
 * the writer types, but nothing exists on disk yet, so there is nothing to read.
 * @type {import('./$types').LayoutLoad}
 */
export const load = async ({ params, url }) => {
  const fileName = params.lexical;

  if (url.searchParams.has('new')) {
    return {
      history: null,
      fileName,
      content: '',
      isDraft: true,
    };
  }

  const content = await invoke('read_file', { name: fileName });
  const history = await invoke('get_file_history', { name: fileName });

  return {
    history,
    fileName,
    content,
    isDraft: false,
  };
};
