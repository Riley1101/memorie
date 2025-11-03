import { invoke } from '@tauri-apps/api/core';

/** @type {import('./$types').LayoutLoad} */
export const load = async ({ params }) => {
  const fileName = params.lexical;
  const content = await invoke('read_file', { name: fileName });

  const history = await invoke('get_file_history', { name: fileName });
  return {
    history,
    fileName,
    content,
  };
};
