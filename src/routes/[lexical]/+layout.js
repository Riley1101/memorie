import { invoke } from '@tauri-apps/api/core';

/** @type {import('./$types').LayoutLoad} */
export const load = async ({ params }) => {
  const fileName = params.lexical;
  const content = await invoke('read_file', { name: fileName });
  const history = await invoke('get_file_history', { name: fileName });
  const dirty_chunks = await invoke('get_document_context', { name: fileName });
  console.log('Dirty Chunks:', dirty_chunks);

  return {
    history,
    fileName,
    content,
  };
};
